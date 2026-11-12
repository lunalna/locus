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

package org.geogebra.web.full.gui.dialog;

import org.geogebra.common.ownership.GlobalScope;
import org.geogebra.web.html5.gui.BaseWidgetFactory;
import org.geogebra.web.html5.gui.GPopupPanel;
import org.geogebra.web.html5.main.AppW;
import org.gwtproject.event.dom.client.ClickEvent;
import org.gwtproject.user.client.ui.FlowPanel;
import org.gwtproject.user.client.ui.HTML;
import org.gwtproject.user.client.ui.Widget;

import elemental2.core.Function;
import elemental2.dom.DomGlobal;
import jsinterop.base.Js;

/**
 * Dropdown of the desktop app's logo that switches the window to another app.
 */
public final class LocusAppPopup extends GPopupPanel {

	private static final String[][] APPS = {{"classic", "Classic"}, {"suite", "Suite"},
			{"graphing", "Graphing"}, {"geometry", "Geometry"}, {"3d", "3D"}, {"cas", "CAS"},
			{"scientific", "Scientific"}, {"notes", "Whiteboard"}};

	private final Widget anchor;

	/**
	 * @param app application
	 * @param anchor the logo
	 */
	public LocusAppPopup(AppW app, Widget anchor) {
		super(true, app.getAppletFrame(), app);
		this.anchor = anchor;
		addAutoHidePartner(anchor.getElement());
		addStyleName("appPickerPopup");
		FlowPanel content = new FlowPanel();
		String current = app.getConfig().getAppCode();
		for (String[] entry : APPS) {
			if (entry[0].equals(current)) {
				continue;
			}
			FlowPanel row = new FlowPanel();
			row.setStyleName("appPickerRow");
			HTML icon = new HTML(Js.asPropertyMap(Js.asPropertyMap(DomGlobal.window)
					.get("locusAppIcons")).get(entry[0]).toString());
			icon.setStyleName("appIcon");
			row.add(icon);
			row.add(BaseWidgetFactory.INSTANCE.newPrimaryText("Locus " + entry[1],
					"appPickerLabel"));
			row.addDomHandler(event -> {
				hide();
				Js.<Function>uncheckedCast(Js.asPropertyMap(DomGlobal.window)
						.get("locusSwitchApp")).call(null, entry[0]);
			}, ClickEvent.getType());
			content.add(row);
		}
		add(content);
		app.registerAutoclosePopup(this);
	}

	/**
	 * @return whether the page runs in the desktop app
	 */
	public static boolean isAvailable() {
		return Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("locusSwitchApp"));
	}

	/**
	 * Shows or hides the popup; apps cannot be switched during an exam.
	 */
	public void toggle() {
		if (isShowing()) {
			hide();
		} else if (!GlobalScope.isExamActive(app)) {
			showRelativeTo(anchor);
		}
	}
}
