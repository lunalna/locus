/*
 * GeoGebra - Dynamic Mathematics for Everyone
 * Copyright (c) GeoGebra GmbH, Altenbergerstr. 69, 4040 Linz, Austria
 * https://www.geogebra.org
 *
 * This file is licensed by GeoGebra GmbH under the EUPL 1.2 licence and
 * may be used under the EUPL 1.2 in compatible projects (see Article 5
 * and the Appendix of EUPL 1.2 for details).
 * You may obtain a copy of the licence at:
 * https://interoperable-europe.ec.europa.eu/collection/eupl/eupl-text-eupl-12
 *
 * Note: The overall GeoGebra software package is free to use for
 * non-commercial purposes only.
 * See https://www.geogebra.org/license for full licensing details
 */

package org.geogebra.web.html5.main;

import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.function.Supplier;

import org.geogebra.common.awt.GColor;
import org.geogebra.common.awt.GDimension;
import org.geogebra.common.awt.GFont;
import org.geogebra.common.awt.GGraphics2D;
import org.geogebra.common.awt.ThemeColors;
import org.geogebra.common.euclidian.DrawEquation;
import org.geogebra.common.kernel.geos.TextProperties;
import org.geogebra.common.kernel.kernelND.GeoElementND;
import org.geogebra.common.main.App;
import org.geogebra.ggbjdk.java.awt.geom.Dimension;
import org.geogebra.web.awt.GGraphics2DW;
import org.geogebra.web.awt.JLMContext2D;
import org.geogebra.web.awt.JLMContextHelper;
import org.gwtproject.canvas.client.Canvas;
import org.gwtproject.dom.style.shared.Unit;
import org.jspecify.annotations.NonNull;

import com.himamis.retex.renderer.share.TeXFont;
import com.himamis.retex.renderer.share.TeXIcon;
import com.himamis.retex.renderer.share.platform.graphics.Image;
import com.himamis.retex.renderer.web.FactoryProviderGWT;
import com.himamis.retex.renderer.web.graphics.Graphics2DW;

import elemental2.dom.DOMMatrix;
import elemental2.dom.DomGlobal;
import elemental2.dom.HTMLCanvasElement;
import jsinterop.base.Js;

/**
 * Web LaTeX helper class
 */
public class DrawEquationW extends DrawEquation {
	/*
	 * needed for avoid the pixelated appearance of LaTeX texts at printing
	 */
	private static double printScale = 1;
	private final Supplier<Double> pixelRatio;
	private static final int MAX_BITMAP_PIXELS = 8_000_000;
	private static final int MAX_BITMAPS = 2000;
	private static final int BITMAP_PADDING = 4;
	private final LinkedHashMap<String, CachedEquation> bitmaps =
			new LinkedHashMap<String, CachedEquation>(64, 0.75f, true) {
				@Override
				protected boolean removeEldestEntry(Map.Entry<String, CachedEquation> eldest) {
					if (size() <= MAX_BITMAPS) {
						return false;
					}
					if (eldest.getValue().canvas != null) {
						HTMLCanvasElement canvas = eldest.getValue().canvas;
						bitmapPixels -= canvas.width * canvas.height;
					}
					return true;
				}
			};
	private int bitmapPixels;
	private int themeVersion;

	private static class CachedEquation {
		HTMLCanvasElement canvas;
		int width;
		int height;
		boolean tooLarge;
	}

	public DrawEquationW(Supplier<Double> pixelRatio) {
		this.pixelRatio = pixelRatio;
	}

	@Override
	public GDimension drawEquation(
			App app1,
			GeoElementND geo,
			final GGraphics2D g2,
			int x,
			int y,
			String eqstring,
			GFont font,
			boolean serif,
			final GColor fgColor,
			GColor bgColor,
			boolean useCache,
			boolean updateAgain,
			final Runnable callback) {

		GGraphics2DW graphics = (GGraphics2DW) g2;
		JLMContext2D ctx = graphics.getContext();
		int style = getLaTeXStyle(font, serif);
		String key = null;
		DOMMatrix transform = null;
		if (!app1.isExporting() && graphics.getCanvas() != null && !eqstring.contains("graphics")
				&& !eqstring.contains("imagebase")) {
			transform = ctx.getTransform();
			if (transform.b == 0 && transform.c == 0 && transform.a == transform.d
					&& transform.a > 0) {
				key = eqstring + '\0' + font.getSize() + '\0' + style + '\0' + fgColor.getARGB()
						+ '\0' + transform.a + '\0' + ctx.locusThemed;
			}
		}
		if (key != null) {
			CachedEquation cached = findCached(key);
			if (cached == null) {
				bitmaps.put(key, new CachedEquation());
			} else {
				if (cached.canvas == null && !cached.tooLarge) {
					renderBitmap(cached, eqstring, fgColor, font, style, transform.a,
							ctx.locusThemed);
				}
				if (cached.canvas != null) {
					ctx.save();
					ctx.setTransform(1, 0, 0, 1, 0, 0);
					ctx.drawImage(cached.canvas,
							Math.round((x - BITMAP_PADDING) * transform.a + transform.e),
							Math.round((y - BITMAP_PADDING) * transform.a + transform.f));
					ctx.restore();
					if (callback != null) {
						callback.run();
					}
					return new Dimension(cached.width, cached.height);
				}
			}
		}

		TeXIcon icon = createIcon(eqstring, fgColor, font, style, null, null);

		Graphics2DW g3 = new Graphics2DW(ctx);
		g3.setDrawingFinishedCallback(async -> {
			g2.updateCanvasColor();
			if (callback != null) {
				callback.run();
			}
		});
		icon.paintIcon(() -> fgColor, g3, x, y);
		g2.updateCanvasColor();
		g3.maybeNotifyDrawingFinishedCallback(false);
		return new Dimension(icon.getIconWidth(), icon.getIconHeight());
	}

	private CachedEquation findCached(String key) {
		if (themeVersion != ThemeColors.getVersion()) {
			themeVersion = ThemeColors.getVersion();
			bitmaps.clear();
			bitmapPixels = 0;
		}
		return bitmaps.get(key);
	}

	// rendered when a formula is drawn a second time, so changing texts skip it
	private void renderBitmap(CachedEquation cached, String eqstring, GColor fgColor, GFont font,
			int style, double scale, boolean themed) {
		TeXIcon icon = createIcon(eqstring, fgColor, font, style, null, null);
		int width = (int) Math.ceil((icon.getIconWidth() + 2 * BITMAP_PADDING) * scale);
		int height = (int) Math.ceil((icon.getIconHeight() + 2 * BITMAP_PADDING) * scale);
		if (width * (double) height > MAX_BITMAP_PIXELS / 4) {
			cached.tooLarge = true;
			return;
		}
		HTMLCanvasElement canvas = Js.uncheckedCast(DomGlobal.document.createElement("canvas"));
		canvas.width = width;
		canvas.height = height;
		JLMContext2D ctx = JLMContextHelper.as(canvas.getContext("2d"));
		ctx.locusThemed = themed;
		ctx.scale(scale, scale);
		Graphics2DW g3 = new Graphics2DW(ctx);
		boolean[] finished = new boolean[1];
		g3.setDrawingFinishedCallback(async -> finished[0] = true);
		icon.paintIcon(() -> fgColor, g3, BITMAP_PADDING, BITMAP_PADDING);
		g3.maybeNotifyDrawingFinishedCallback(false);
		if (!finished[0]) {
			return;
		}
		cached.canvas = canvas;
		cached.width = icon.getIconWidth();
		cached.height = icon.getIconHeight();
		bitmapPixels += width * height;
		Iterator<CachedEquation> oldest = bitmaps.values().iterator();
		while (bitmapPixels > MAX_BITMAP_PIXELS && oldest.hasNext()) {
			CachedEquation removed = oldest.next();
			if (removed != cached) {
				if (removed.canvas != null) {
					bitmapPixels -= removed.canvas.width * removed.canvas.height;
				}
				oldest.remove();
			}
		}
	}

	/**
	 * @param t
	 *            print scale
	 */
	public static void setPrintScale(double t) {
		printScale = t;
	}

	/**
	 * @param geo
	 *            geo element, used for color and app
	 * @param text0
	 *            LaTeX text
	 * @param c0
	 *            canvas (may be null)
	 * @param fontSize
	 *            font size
	 * @return canvas
	 */
	public static Canvas paintOnCanvas(GeoElementND geo, String text0, Canvas c0, double fontSize) {
		Canvas c = makeCleanCanvas(c0);
		if (geo != null) {
			DrawEquationW current = (DrawEquationW) geo.getKernel().getApplication().getDrawEquation();
			current.paintOnCleanCanvas(text0, c, fontSize, GColor.BLACK, needsSerif(geo));
		}
		return c;
	}

	/**
	 * @param geo
	 *            element
	 * @param text0
	 *            text
	 * @param c0
	 *            canvas or null
	 * @param fontSize
	 *            font size
	 * @return canvas
	 */
	public static Canvas paintOnCanvasOutput(
			GeoElementND geo, String text0, Canvas c0, double fontSize) {
		final GColor fgColor = geo.getAlgebraColor();
		Canvas c = makeCleanCanvas(c0);
		DrawEquationW current = (DrawEquationW) geo.getKernel().getApplication().getDrawEquation();
		current.paintOnCleanCanvas(text0, c, fontSize, fgColor, needsSerif(geo));
		return c;
	}

	/**
	 * @param geo element
	 * @return whether serif is needed
	 */
	public static boolean needsSerif(GeoElementND geo) {
		boolean serif = false;
		if (geo instanceof TextProperties) {
			serif = ((TextProperties) geo).isSerifFont();
		}
		return serif;
	}

	/**
	 * @param old old canvas
	 * @return cleaned old canvas or a new instance
	 */
	public static Canvas makeCleanCanvas(Canvas old) {
		Canvas c = old;
		if (c == null) {
			c = Canvas.createIfSupported();
		} else {
			c.getContext2d().fillRect(0, 0, c.getCoordinateSpaceWidth(), c.getCoordinateSpaceHeight());
		}
		return c;
	}

	/**
	 * @param text0
	 *            LaTeX text
	 * @param c
	 *            canvas
	 * @param fontSize
	 *            font size
	 * @param fgColor
	 *            color
	 * @param serif
	 *            whether to use serif font
	 * @return graphics
	 */
	public Graphics2DW paintOnCleanCanvas(
			String text0, @NonNull Canvas c, double fontSize, final GColor fgColor, boolean serif) {
		JLMContext2D ctx = JLMContextHelper.as(c.getContext2d());
		ctx.locusThemed = true;

		checkFirstCall();
		TeXIcon icon = createIcon(text0, fgColor, fontSize, serif ? 0 : TeXFont.SANSSERIF);
		Graphics2DW g3 = new Graphics2DW(ctx);

		double ratio = pixelRatio.get() * printScale;
		double width = roundUp(Math.min(icon.getIconWidth(), 20000), ratio);
		double height = roundUp(icon.getIconHeight(), ratio);
		c.setCoordinateSpaceWidth((int) (width * ratio));
		c.setCoordinateSpaceHeight((int) (height * ratio));
		c.getElement().getStyle().setWidth(width, Unit.PX);
		c.getElement().getStyle().setHeight(height, Unit.PX);
		ctx.scale2(ratio, ratio);

		icon.paintIcon(() -> fgColor, g3, 0, 0);
		return g3;
	}

	private static double roundUp(int w, double ratio) {

		return Math.ceil(w * ratio) / ratio;
	}

	@Override
	public GDimension measureEquation(App app, String text, GFont font, boolean serif) {
		return this.measureEquationJLaTeXMath(app, text, font, serif, null, null);
	}

	@Override
	public void checkFirstCall() {
		FactoryProviderGWT.ensureLoaded();
		DrawEquation.checkFirstCallStatic();
	}

	@Override
	public Image getCachedDimensions(
			String text, GeoElementND geo, GColor fgColor, GFont font, int style, int[] ret) {
		// TODO JLaTeXMathCache uses
		// import java.lang.ref.Reference;
		// import java.lang.ref.ReferenceQueue;
		// import java.lang.ref.SoftReference;
		// so can't use this method in web at the moment
		return null;
	}
}
