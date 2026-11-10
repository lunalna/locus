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

package org.geogebra.web.full.gui.view.assistant;

import org.gwtproject.dom.style.shared.Position;
import org.gwtproject.user.client.ui.SimplePanel;

import elemental2.core.Function;
import elemental2.dom.DomGlobal;
import jsinterop.base.Js;

/**
 * Locus AI assistant, whose content is provided by the host page through window.locusAssistant.
 */
public final class AssistantPanel extends SimplePanel {

	/**
	 * Creates the panel
	 */
	public AssistantPanel() {
		setSize("100%", "100%");
		getElement().getStyle().setPosition(Position.RELATIVE);
	}

	/**
	 * @return whether the host page provides the assistant
	 */
	public static boolean isAvailable() {
		return Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("locusAssistant"));
	}

	/**
	 * @return whether the assistant is turned on, set up and allowed, as reported by the host page
	 */
	public static boolean isEnabled() {
		return Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("locusAssistantEnabled"));
	}

	@Override
	protected void onLoad() {
		Js.<Function>uncheckedCast(Js.asPropertyMap(DomGlobal.window).get("locusAssistant"))
				.call(null, getElement());
	}
}
