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

import org.geogebra.web.full.main.AppWFull;
import org.gwtproject.user.client.ui.FlowPanel;

import elemental2.dom.HTMLElement;
import elemental2.dom.PointerEvent;
import jsinterop.base.Js;

/**
 * AI assistant shown right of the views in Classic, resizable by dragging its left edge.
 */
public final class AssistantSidePanel extends FlowPanel {

	private boolean dragging;
	private double startX;
	private int startWidth;

	/**
	 * @param app application
	 */
	public AssistantSidePanel(AppWFull app) {
		addStyleName("assistantSidePanel");
		FlowPanel dragger = new FlowPanel();
		dragger.setStyleName("assistantDragger gwt-SplitLayoutPanel-HDragger splitPaneDragger");
		add(dragger);
		add(new AssistantPanel());

		HTMLElement handle = Js.uncheckedCast(dragger.getElement());
		handle.addEventListener("pointerdown", event -> {
			PointerEvent down = Js.uncheckedCast(event);
			handle.setPointerCapture(down.pointerId);
			dragging = true;
			startX = down.clientX;
			startWidth = app.getAssistantWidth();
		});
		handle.addEventListener("pointermove", event -> {
			if (dragging) {
				PointerEvent move = Js.uncheckedCast(event);
				app.setAssistantWidth((int) (startWidth + startX - move.clientX));
			}
		});
		handle.addEventListener("pointerup", event -> dragging = false);
	}
}
