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

package org.geogebra.web.full.main.embed;

import java.util.HashMap;
import java.util.Map;
import java.util.Map.Entry;

import org.geogebra.common.util.debug.Log;
import org.geogebra.gwtutil.JavaScriptInjector;
import org.geogebra.gwtutil.ScriptLoadCallback;

import elemental2.dom.DomGlobal;
import elemental2.dom.HTMLScriptElement;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 *
 * Loader for Graspable Math
 *
 */
public final class GMLoader {
	static final GMLoader INSTANCE = new GMLoader();
	private static final String ORIGIN = "https://graspablemath.com";
	private final Map<Integer, GraspableEmbedElement> loadQueue = new HashMap<>();
	private boolean loadingStarted;

	private GMLoader() {
		// the scripts loaded by gm-inject.js report no errors, so failed loads are caught here
		DomGlobal.window.addEventListener("error", event -> {
			if (loadingStarted && event.target instanceof HTMLScriptElement
					&& ((HTMLScriptElement) event.target).src.startsWith(ORIGIN)) {
				onError();
			}
		}, true);
	}

	/**
	 * @param graspableEmbedElement
	 *            GM embedded element
	 * @param embedID
	 *            embed ID
	 */
	public void load(GraspableEmbedElement graspableEmbedElement, int embedID) {
		loadQueue.put(embedID, graspableEmbedElement);
		if (loadingStarted) {
			return;
		}
		loadingStarted = true;
		if (Js.asPropertyMap(DomGlobal.window).has("loadGM")) {
			loadLatest();
			return;
		}
		JavaScriptInjector.loadJS(ORIGIN + "/shared/libs/gmath/gm-inject.js",
				new ScriptLoadCallback() {

					@Override
					public void onLoad() {
						loadLatest();
					}

					@Override
					public void onError() {
						GMLoader.this.onError();
					}

					@Override
					public void cancel() {
						// no need to cancel
					}
				});
	}

	private void loadLatest() {
		NativeGMLoader.loadGM(
				this::loadFromQueue, JsPropertyMap.of("version", "latest", "build", "ggb"));
	}

	private void onError() {
		Log.warn("Could not load Graspable Math API");
		loadingStarted = false;
		for (GraspableEmbedElement element : loadQueue.values()) {
			element.showLoadError();
		}
		loadQueue.clear();
	}

	/**
	 * Load all elements in the queue
	 */
	private void loadFromQueue() {
		loadingStarted = false;
		for (Entry<Integer, GraspableEmbedElement> entry : loadQueue.entrySet()) {
			entry.getValue().initCanvas(entry.getKey());
		}
		loadQueue.clear();
	}
}
