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

package org.geogebra.web.full.euclidian;

import java.util.List;
import java.util.function.BiConsumer;
import java.util.stream.Collectors;

import org.geogebra.common.euclidian.EuclidianView;
import org.geogebra.common.kernel.geos.GeoElement;
import org.geogebra.common.kernel.geos.MoveGeos;
import org.geogebra.common.kernel.matrix.Coords;
import org.geogebra.web.html5.main.AppW;
import org.jspecify.annotations.Nullable;

import elemental2.dom.DomGlobal;
import elemental2.dom.EventListener;
import elemental2.dom.PointerEvent;
import jsinterop.base.Js;

/**
 * Moves objects with a pointer that is not handled by the graphics view.
 */
public final class GeoDragger {

	private GeoDragger() {
	}

	/**
	 * Moves the objects with the pointer until it is released.
	 * @param ev graphics view
	 * @param geos objects to move, locked ones stay in place
	 * @param pointerId pointer
	 * @param x client x-coordinate where the drag starts
	 * @param y client y-coordinate where the drag starts
	 * @param onMove called with the moved distance in pixels
	 */
	public static void start(EuclidianView ev, List<GeoElement> geos, int pointerId,
			double x, double y, @Nullable BiConsumer<Double, Double> onMove) {
		AppW app = (AppW) ev.getApplication();
		List<GeoElement> movable =
				geos.stream().filter(geo -> !geo.isLocked()).collect(Collectors.toList());
		double[] last = {x, y};
		EventListener move = event -> {
			PointerEvent e = Js.uncheckedCast(event);
			if (e.pointerId != pointerId) {
				return;
			}
			double zoom = app.getGeoGebraElement().getScaleX();
			double dx = (e.clientX - last[0]) / zoom;
			double dy = (e.clientY - last[1]) / zoom;
			last[0] = e.clientX;
			last[1] = e.clientY;
			MoveGeos.moveObjects(movable,
					new Coords(dx / ev.getXscale(), -dy / ev.getYscale(), 0), null, null, ev);
			app.getKernel().notifyRepaint();
			ev.getEuclidianController().updateBoundingBoxFromSelection(false);
			ev.repaintView();
			if (onMove != null) {
				onMove.accept(dx, dy);
			}
		};
		EventListener[] end = new EventListener[1];
		end[0] = event -> {
			if (Js.<PointerEvent>uncheckedCast(event).pointerId != pointerId) {
				return;
			}
			DomGlobal.window.removeEventListener("pointermove", move, true);
			DomGlobal.window.removeEventListener("pointerup", end[0], true);
			DomGlobal.window.removeEventListener("pointercancel", end[0], true);
			app.storeUndoInfo();
			ev.getEuclidianController().showDynamicStylebar();
		};
		DomGlobal.window.addEventListener("pointermove", move, true);
		DomGlobal.window.addEventListener("pointerup", end[0], true);
		DomGlobal.window.addEventListener("pointercancel", end[0], true);
	}
}
