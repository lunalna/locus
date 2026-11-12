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

package org.geogebra.web.full.gui.menubar;

import org.geogebra.web.full.css.MaterialDesignResources;
import org.geogebra.web.full.gui.menubar.action.LocusSettingsAction;
import org.geogebra.web.full.main.AppWFull;
import org.geogebra.web.html5.main.AppW;
import org.geogebra.web.html5.main.LocalizationW;
import org.geogebra.web.resources.SVGResource;

/**
 * Single item submenu with settings action
 */
public final class SettingsMenu extends Submenu {

	private boolean shift;

	/**
	 * @param app
	 *            application
	 */
	public SettingsMenu(AppW app) {
		super("settings", app);
	}

	@Override
	public SVGResource getImage() {
		return MaterialDesignResources.INSTANCE.gear();
	}

	@Override
	protected String getTitleTranslationKey() {
		return "Settings";
	}

	@Override
	public String getTitle(LocalizationW localization) {
		return shift ? "Locus settings" : super.getTitle(localization);
	}

	/**
	 * @param shift whether shift is held while the classic settings panel is chosen
	 */
	void setShift(boolean shift) {
		this.shift = shift;
	}

	@Override
	public void handleHeaderClick() {
		open(false);
	}

	/**
	 * Opens the classic settings panel when it is chosen and shift is not held, else the Locus settings.
	 * @param shiftKey whether shift was held
	 */
	void open(boolean shiftKey) {
		if (LocusSettingsAction.useClassic() && !shiftKey) {
			((AppWFull) getApp()).getCurrentActivity().showSettingsView(getApp());
		} else {
			LocusSettingsAction.open(null);
		}
	}
}
