package org.geogebra.web.full.main;

import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;

import org.geogebra.common.awt.ThemeColors;
import org.geogebra.common.main.color.AccentPalette;
import org.geogebra.editor.web.MathFieldW;
import org.geogebra.gwtutil.JsRunnable;
import org.geogebra.web.full.gui.toolbarpanel.ToolbarPanel;
import org.geogebra.web.html5.GeoGebraGlobal;
import org.geogebra.web.resources.SVGResourcePrototype;
import org.geogebra.web.resources.ThemeStyles;

import elemental2.dom.DomGlobal;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 * Applies a theme without reloading the page.
 */
final class ThemeSwitcher {

	private ThemeSwitcher() {
	}

	/**
	 * Exposes window.locusApplyTheme, which the settings call after they changed
	 * window.locusTheme and window.resourcePackColors.
	 * @param app application
	 */
	static void register(AppWFull app) {
		Js.asPropertyMap(DomGlobal.window).set("locusApplyTheme",
				(JsRunnable) () -> apply(app));
	}

	private static void apply(AppWFull app) {
		JsPropertyMap<String> theme = GeoGebraGlobal.locusTheme;
		ThemeColors.set(theme == null ? -1 : parseHex(theme.get("background")),
				theme == null ? 0 : parseHex(theme.get("foreground")));
		JsPropertyMap<String> colors = GeoGebraGlobal.resourcePackColors;
		Set<Integer> kept = new HashSet<>();
		colors.forEach(name -> kept.add(parseHex(colors.get(name))));
		// the color constants keep the accents from startup, so those are replaced on screen
		Map<Integer, Integer> accents = new HashMap<>();
		AccentPalette.DEFAULTS.forEach((shade, fallback) -> {
			String color = colors.get("primary-" + shade);
			int rgb = color == null ? fallback : parseHex(color);
			if (rgb != AccentPalette.get(shade)) {
				accents.put(AccentPalette.get(shade), rgb);
			}
		});
		ThemeColors.setAccents(accents, kept);
		ThemeStyles.update();
		SVGResourcePrototype.updateTheme();
		app.refreshViews();
		if (app.getGuiManager() != null) {
			if (app.getGuiManager().hasAlgebraView()) {
				app.getAlgebraView().updateTheme();
			}
			ToolbarPanel toolbar = app.getGuiManager().getUnbundledToolbar();
			if (toolbar != null) {
				toolbar.repaintSpreadsheet();
			}
		}
		MathFieldW.repaintAll();
	}

	private static int parseHex(String color) {
		return Integer.parseInt(color.substring(1), 16);
	}
}
