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

package org.geogebra.web.full.gui.menubar.action;

import org.geogebra.web.full.gui.menubar.DefaultMenuAction;
import org.geogebra.web.full.main.AppWFull;

import elemental2.core.Function;
import elemental2.dom.DomGlobal;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 * Opens another window of the desktop app running the same app.
 */
public final class LocusWindowAction extends DefaultMenuAction<AppWFull> {

	@Override
	public void execute(AppWFull app) {
		JsPropertyMap<Object> window = Js.asPropertyMap(DomGlobal.window);
		// Classic pages do not set appID.
		Object appID = window.has("appID") ? window.get("appID") : "classic";
		Js.<Function>uncheckedCast(window.get("locusOpenApp")).call(null, appID);
	}
}
