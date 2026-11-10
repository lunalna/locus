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

package org.geogebra.web.full.gui.toolbarpanel;

import org.geogebra.common.io.layout.DockPanelData;
import org.geogebra.web.full.gui.view.assistant.AssistantPanel;

/**
 * Side panel tab for the Locus AI assistant in Suite.
 */
public final class AssistantTab extends ToolbarTab {

	private final ToolbarPanel toolbarPanel;

	/**
	 * @param toolbarPanel parent toolbar panel
	 */
	public AssistantTab(ToolbarPanel toolbarPanel) {
		super(toolbarPanel);
		this.toolbarPanel = toolbarPanel;
		add(new AssistantPanel());
	}

	@Override
	protected void onActive() {
		// the content is provided by the host page
	}

	@Override
	public DockPanelData.TabIds getID() {
		return DockPanelData.TabIds.ASSISTANT;
	}

	@Override
	public void setLabels() {
		// the content is provided by the host page
	}

	@Override
	public void open() {
		toolbarPanel.openAssistant(true);
	}

	@Override
	public void close() {
		toolbarPanel.close(false);
	}
}
