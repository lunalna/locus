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

import org.geogebra.web.full.gui.LocusPluginEntries;
import org.geogebra.web.full.gui.menu.icons.DefaultMenuIconResources;
import org.geogebra.web.full.gui.menubar.action.LocusPluginsAction;
import org.geogebra.web.full.main.AppWFull;
import org.geogebra.web.html5.gui.view.ImageIconSpec;
import org.geogebra.web.html5.main.AppW;
import org.geogebra.web.resources.SVGResource;

/**
 * Classic menu entry that opens the Locus plugin command list, or lists the commands plugins add to the menu.
 */
public final class LocusPluginsMenu extends Submenu {

	/**
	 * @param app application
	 */
	public LocusPluginsMenu(AppW app) {
		super("locusPlugins", app);
		update();
	}

	@Override
	public void update() {
		clearItems();
		for (LocusPluginEntries.Entry entry : LocusPluginEntries.get("menu")) {
			addItem(entry.title, new DefaultMenuAction<>() {
				@Override
				public void execute(AppWFull app) {
					LocusPluginEntries.run(entry, null);
				}
			}, LocusPluginEntries.icon(entry));
		}
		if (!isEmpty()) {
			addItem("All plugin commands", new LocusPluginsAction(), new ImageIconSpec(getImage()));
		}
	}

	@Override
	public SVGResource getImage() {
		return DefaultMenuIconResources.INSTANCE.plugins();
	}

	@Override
	protected String getTitleTranslationKey() {
		return "Plugins";
	}

	@Override
	public void handleHeaderClick() {
		LocusPluginsAction.open();
	}
}
