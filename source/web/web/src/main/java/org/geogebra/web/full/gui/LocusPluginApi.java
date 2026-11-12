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

package org.geogebra.web.full.gui;

import java.util.ArrayList;
import java.util.List;

import org.geogebra.common.awt.GPoint;
import org.geogebra.common.euclidian.EuclidianView;
import org.geogebra.common.euclidian.event.PointerEventType;
import org.geogebra.common.kernel.geos.GeoElement;
import org.geogebra.web.html5.main.AppW;

import elemental2.core.JsArray;
import elemental2.dom.DomGlobal;
import jsinterop.annotations.JsFunction;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/** Selection and hit testing for plugins, which the apps API does not offer. */
public final class LocusPluginApi {

	@JsFunction
	private interface Names {
		JsArray<String> get();
	}

	@JsFunction
	private interface Select {
		void select(JsArray<String> names);
	}

	@JsFunction
	private interface Hits {
		JsArray<String> get(double x, double y);
	}

	private LocusPluginApi() {
	}

	/**
	 * Adds locusPluginSelection, locusPluginSelect and locusPluginHits to the page.
	 * @param app app
	 */
	public static void register(AppW app) {
		JsPropertyMap<Object> window = Js.asPropertyMap(DomGlobal.window);
		window.set("locusPluginSelection",
				(Names) () -> names(app.getSelectionManager().getSelectedGeos()));
		window.set("locusPluginSelect", (Select) names -> {
			List<GeoElement> geos = new ArrayList<>();
			for (int i = 0; i < names.length; i++) {
				GeoElement geo = app.getKernel().lookupLabel(names.getAt(i));
				if (geo != null) {
					geos.add(geo);
				}
			}
			app.getSelectionManager().setSelectedGeos(geos);
		});
		// x and y are in page pixels
		window.set("locusPluginHits", (Hits) (x, y) -> {
			EuclidianView view = app.getEuclidianView1();
			view.setHits(new GPoint((int) x - view.getAbsoluteLeft(),
					(int) y - view.getAbsoluteTop()), PointerEventType.MOUSE);
			return names(view.getHits());
		});
	}

	private static JsArray<String> names(List<? extends GeoElement> geos) {
		JsArray<String> names = new JsArray<>();
		for (GeoElement geo : geos) {
			names.push(geo.getLabelSimple());
		}
		return names;
	}
}
