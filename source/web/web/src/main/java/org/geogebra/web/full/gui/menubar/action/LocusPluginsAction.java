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

import org.geogebra.gwtutil.JsRunnable;
import org.geogebra.web.full.gui.menubar.DefaultMenuAction;
import org.geogebra.web.full.main.AppWFull;

import elemental2.dom.DomGlobal;
import jsinterop.base.Js;

/**
 * Opens the plugin command list provided by the desktop app, which the web version does not have.
 */
public final class LocusPluginsAction extends DefaultMenuAction<AppWFull> {

	@Override
	public void execute(AppWFull app) {
		open();
	}

	/**
	 * Opens the plugin command list.
	 */
	public static void open() {
		Object open = Js.asPropertyMap(DomGlobal.window).get("locusPlugins");
		if (open != null) {
			Js.<JsRunnable>uncheckedCast(open).run();
		}
	}
}
