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

import org.geogebra.gwtutil.FileSystemAPI;
import org.geogebra.gwtutil.FileSystemFileHandle;
import org.geogebra.web.full.gui.menubar.DefaultMenuAction;
import org.geogebra.web.full.gui.menubar.FileChooser;
import org.geogebra.web.full.main.AppWFull;
import org.geogebra.web.full.main.FileManager;
import org.geogebra.web.html5.gui.util.BrowserStorage;

public final class OpenOfflineFileAction extends DefaultMenuAction<AppWFull> {

	private FileChooser fileChooser;

	@Override
	public void execute(final AppWFull app) {
		if (FileSystemAPI.isOpenSupported()) {
			app.checkSaved(saved -> openWithPicker(app));
		} else if (!app.getLoginOperation().isLoggedIn()) {
			BrowserStorage.SESSION.setItem("saveAction", "openOfflineFile");
			app.getSaveController().showDialogIfNeeded(obj -> onOpenFile(app), false);
		} else {
			onOpenFile(app);
		}
	}

	private void openWithPicker(AppWFull app) {
		FileSystemAPI.showOpenFilePicker().then(handles -> {
			FileSystemFileHandle handle = handles.getAt(0);
			handle.getFile().then(file -> {
				if (app.openFile(file) && file.name.endsWith(app.getFileExtension())) {
					((FileManager) app.getFileManager()).setFileHandle(handle);
				}
				return null;
			});
			return null;
		});
	}

	private void onOpenFile(final AppWFull app) {
		if (fileChooser == null) {
			fileChooser = new FileChooser(app);
			fileChooser.addStyleName("hidden");
		}
		app.getAppletFrame().add(fileChooser);
		fileChooser.open();
	}
}
