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

package org.geogebra.web.html5.main;

import org.geogebra.web.html5.GeoGebraGlobal;

/**
 * Skips animation frames to keep the frame rate limit of the desktop app's performance settings.
 */
public class FrameLimiter {

	private final double interval;
	private double last;

	/**
	 * Create a limiter for the configured frame rate.
	 */
	public FrameLimiter() {
		double fps = GeoGebraGlobal.locusPerformance == null ? 0
				: GeoGebraGlobal.locusPerformance.get("fps");
		interval = fps > 0 ? 1000 / fps : 0;
	}

	/**
	 * @param timestamp animation frame timestamp in ms
	 * @return whether this frame should be used
	 */
	public boolean ready(double timestamp) {
		double elapsed = timestamp - last;
		// a millisecond of tolerance for frames that arrive slightly early
		if (elapsed < interval - 1) {
			return false;
		}
		// keeps the average rate when the display rate is not a multiple of the limit
		last = interval > 0 && elapsed < 2 * interval ? timestamp - elapsed % interval : timestamp;
		return true;
	}
}
