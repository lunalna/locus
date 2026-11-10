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

import org.geogebra.gwtutil.JsConsumer;

import elemental2.dom.DomGlobal;
import jsinterop.base.Js;

/**
 * Opens the Locus settings modal provided by the desktop app.
 */
public final class LocusSettingsAction {

	private LocusSettingsAction() {
	}

	/**
	 * Opens the Locus settings modal.
	 * @param section raw name of the settings tab to show, or null for the first section
	 */
	public static void open(String section) {
		Js.<JsConsumer<String>>uncheckedCast(
				Js.asPropertyMap(DomGlobal.window).get("locusSettings")).accept(section);
	}
}
