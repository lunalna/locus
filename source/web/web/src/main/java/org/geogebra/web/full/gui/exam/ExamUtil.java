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

package org.geogebra.web.full.gui.exam;

import java.util.ArrayList;
import java.util.List;

import org.geogebra.common.awt.GColor;
import org.geogebra.common.exam.ExamController;
import org.geogebra.common.ownership.GlobalScope;
import org.geogebra.common.ownership.SuiteScope;
import org.geogebra.common.util.StringUtil;
import org.geogebra.gwtutil.JsConsumer;
import org.geogebra.gwtutil.JsRunnable;
import org.geogebra.web.html5.Browser;
import org.geogebra.web.html5.GeoGebraGlobal;
import org.geogebra.web.html5.gui.util.Dom;
import org.geogebra.web.html5.main.AppW;
import org.gwtproject.dom.client.Element;

import elemental2.core.Function;
import elemental2.dom.DomGlobal;
import jsinterop.base.Js;

/**
 * Utility class for exam mode
 */
public final class ExamUtil {

	private AppW app;
	private static boolean examModeRunning = false;
	private static boolean windowLeavingAllowed = false;
	private static final List<String> hostLog = new ArrayList<>();
	private static List<String> hostStatus = new ArrayList<>();
	private final ExamController examController;
	private boolean hostFocused = true;
	private boolean hostFullscreen = true;

	/**
	 * @param app
	 *            application
	 */
	public ExamUtil(AppW app) {
		this.app = app;
		this.examController = GlobalScope.getExamController(app);
	}

	/**
	 * check and log window resize and focus lost/gained window resize is
	 * checked first - if window is not in full screen mode "cheating" can't be
	 * stopped (only going back to full screen ends "cheating") if window is in
	 * full screen losing focus starts "cheating", gaining focus stops
	 * "cheating"
	 *
	 * @param tabletMode
	 *            whether we are in tablet app
	 */
	private void addVisibilityAndBlurHandlers(boolean tabletMode) {
		if (hasExternalSecurityCheck(app)) {
			return;
		}
		boolean fromHost = Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("ggbExamFocusFromHost"));
		if (tabletMode && !fromHost) {
			app.getGlobalHandlers().addEventListener(DomGlobal.document, "visibilitychange", (e) -> {
				if (Js.isTruthy(DomGlobal.document.hidden)) {
					startCheating();
				} else {
					stopCheating();
				}
			});
		} else {
			if (!fromHost) {
				app.getGlobalHandlers().addEventListener(DomGlobal.window, "blur", (event) -> {
					// The focusout event should not be caught:
					if ("blur".equals(event.type)) {
						startCheating();
					}
				});
				app.getGlobalHandlers().addEventListener(DomGlobal.window, "focus", (event) -> {
					onWindowFocused();
				});
			}
			// called by the desktop host when another app gets or releases keyboard focus
			Js.asPropertyMap(DomGlobal.window).set("ggbExamWindowFocus",
					(JsConsumer<Boolean>) focused -> {
						hostFocused = focused;
						if (focused) {
							onWindowFocused();
						} else {
							startCheating();
						}
					});
			Js.asPropertyMap(DomGlobal.window).set("ggbExamFullscreenChanged",
					(JsConsumer<Boolean>) fullscreen -> {
						hostFullscreen = fullscreen;
						if (!fullscreen) {
							startCheating();
						} else if (hostFocused) {
							stopCheating();
						}
					});
			// called by the desktop host for plugins
			Js.asPropertyMap(DomGlobal.window).set("ggbExamLog", (JsConsumer<String>) text -> {
				if (examController.isExamActive()) {
					hostLog.add(examController.getDurationFormatted(app.getLocalization()) + " " + text);
				}
			});
			Js.asPropertyMap(DomGlobal.window).set("ggbExamStatus",
					(JsConsumer<String[]>) lines -> hostStatus = List.of(lines));
			Js.asPropertyMap(DomGlobal.window).set("ggbExamAllowLeaving",
					(JsConsumer<Boolean>) allowed -> setWindowLeavingAllowed(allowed));
			if (!fromHost) {
				app.getGlobalHandlers().addEventListener(DomGlobal.window, "resize", (evt) -> {
					boolean fullscreen = Browser.isCoveringWholeScreen();
					if (!fullscreen) {
						startCheating();
					} else {
						stopCheating();
					}
				});
			}
		}
	}

	/**
	 * @return whether we're running in a secure browser
	 */
	public static boolean hasExternalSecurityCheck(AppW app) {
		return app.isLockedExam() && app.isSecuredBrowser();
	}

	private void onWindowFocused() {
		boolean fromHost = Js.isTruthy(Js.asPropertyMap(DomGlobal.window).get("ggbExamFocusFromHost"));
		if (fromHost ? hostFullscreen : Browser.isCoveringWholeScreen()) {
			stopCheating();
		}
	}

	private void startCheating() {
		if (examController.isExamActive() && !windowLeavingAllowed) {
			examController.getCheatingEvents().addWindowLeftEvent();
		}
	}

	private void stopCheating() {
		if (examController.isExamActive()) {
			examController.getCheatingEvents().addWindowEnteredEvent();
		}
	}

	/**
	 * @param full
	 *            whether to switch to fullscreen
	 */
	public static void toggleFullscreen(boolean full) {
		if (GeoGebraGlobal.getGgbExamMode() != null && (full == !examModeRunning)) {
			examModeRunning = full;
			GeoGebraGlobal.getGgbExamMode().accept(full);
		}
	}

	/**
	 * Clears the plugin log and status, to be called when the exam starts.
	 */
	public static void resetHostLog() {
		windowLeavingAllowed = false;
		hostLog.clear();
		hostStatus = new ArrayList<>();
	}

	/**
	 * @return plugin status lines
	 */
	public static List<String> getHostStatus() {
		return hostStatus;
	}

	/**
	 * @return plugin log entries, with the exam time
	 */
	public static List<String> getHostLog() {
		return hostLog;
	}

	// Disallowing waits until the host is back in lockdown, so the transition is not counted as leaving.
	private static void setWindowLeavingAllowed(boolean allowed) {
		Object lockdown = Js.asPropertyMap(DomGlobal.window).get("ggbExamLockdown");
		if (allowed || !Js.isTruthy(lockdown)) {
			windowLeavingAllowed = allowed;
		}
		if (Js.isTruthy(lockdown)) {
			Js.<Function>uncheckedCast(lockdown).call(null, !allowed,
					(JsRunnable) () -> windowLeavingAllowed = allowed);
		}
	}

	/**
	 * Listen to focus / blur / resize events on the browser window.
	 */
	public void addVisibilityAndBlurHandlers() {
		addVisibilityAndBlurHandlers(app.getLAF().isTablet());
	}

	/**
	 * @param element
	 *            element
	 * @param red
	 *            whether it should be red
	 */
	public static void makeRed(Element element, boolean red) {
		if (red) {
			Dom.setImportant(
					element.getStyle(), "background-color", StringUtil.toHtmlColor(GColor.DARK_RED));
		} else {
			element.getStyle().setBackgroundColor("");
		}
	}

	/**
	 * @param appW
	 *            app
	 * @return exam status description
	 */
	public static String status(AppW appW) {
		SuiteScope suiteScope = GlobalScope.getSuiteScope(appW);
		ExamController examController = suiteScope != null ? suiteScope.examController : null;
		return appW.getLocalization().getMenu("exam_menu_entry") + ": "
				+ (examController != null && examController.isCheating()
						? appW.getLocalization().getMenu("exam_alert")
						: appW.getLocalization().getMenu("OK"));
	}
}
