package org.geogebra.web.full.gui.exam;

import org.geogebra.common.cas.view.CASView;
import org.geogebra.common.kernel.StringTemplate;
import org.geogebra.common.kernel.geos.GeoCasCell;
import org.geogebra.common.kernel.kernelND.GeoElementND;
import org.geogebra.common.main.App;
import org.geogebra.web.full.gui.GuiManagerW;
import org.geogebra.web.html5.main.AppW;

import elemental2.dom.DomGlobal;
import jsinterop.annotations.JsFunction;
import jsinterop.base.Js;

/** Lets the desktop host's AI assistant add rows to the CAS view. */
public final class AssistantBridge {

	@JsFunction
	private interface CasRow {
		String add(String input);
	}

	@JsFunction
	private interface CasEnabled {
		boolean get();
	}

	private AssistantBridge() {
	}

	/**
	 * Registers window.ggbApplet, window.ggbAssistantCasEnabled and window.ggbAssistantCasRow.
	 * @param app application
	 */
	public static void register(AppW app) {
		// creating the script manager exports window.ggbApplet, which the assistant tools use
		app.getScriptManager();
		Js.asPropertyMap(DomGlobal.window).set("ggbAssistantCasEnabled",
				(CasEnabled) () -> app.getSettings().getCasSettings().isEnabled());
		Js.asPropertyMap(DomGlobal.window).set("ggbAssistantCasRow",
				(CasRow) input -> addCasRow(app, input));
		if (app.getSettings().getCasSettings().isEnabled()) {
			// the CAS engine loads in the background, so start it before the first question
			app.getGgbApi().evalCommandCAS("1+1", null);
		}
	}

	private static String addCasRow(AppW app, String input) {
		GuiManagerW guiManager = (GuiManagerW) app.getGuiManager();
		guiManager.setShowView(true, App.VIEW_CAS);
		CASView view = guiManager.getCasView();
		int last = view.getRowCount() - 1;
		int row = view.isRowEmpty(last) ? last : last + 1;
		GeoCasCell cell = new GeoCasCell(app.getKernel().getConstruction());
		cell.setInput(input);
		view.insertRow(cell, false);
		view.getInputHandler().processRowThenEdit(row, false, null);
		// rows stay in the CAS; plots are left to the student
		GeoElementND twin = cell.getTwinGeo();
		if (twin != null) {
			twin.setEuclidianVisible(false);
			twin.updateRepaint();
		}
		app.storeUndoInfo();
		String output = cell.getOutput(StringTemplate.defaultTemplate);
		if (output.isEmpty()) {
			return "Error: Locus could not evaluate this input.";
		}
		return cell.isError() ? "Error: " + output : output;
	}
}
