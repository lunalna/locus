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

import org.geogebra.common.kernel.AnimationManager;
import org.geogebra.common.kernel.Kernel;

import elemental2.dom.DomGlobal;
import elemental2.dom.FrameRequestCallback;

/**
 * Animation manager that steps once per display frame.
 */
public class AnimationManagerW extends AnimationManager {

	private final FrameLimiter limiter = new FrameLimiter();
	private final FrameRequestCallback frame = this::step;
	private boolean running;
	private boolean scheduled;

	/**
	 * @param kernel kernel
	 */
	public AnimationManagerW(Kernel kernel) {
		super(kernel);
	}

	private void step(double timestamp) {
		scheduled = false;
		if (running) {
			if (limiter.ready(timestamp)) {
				sliderStep();
			}
			schedule();
		}
	}

	private void schedule() {
		if (!scheduled) {
			scheduled = true;
			DomGlobal.requestAnimationFrame(frame);
		}
	}

	@Override
	public boolean isRunning() {
		return running;
	}

	@Override
	protected void setTimerDelay(int delay) {
		// the display sets the pace
	}

	@Override
	protected void startTimer() {
		running = true;
		schedule();
		kernel.getApplication().getEventDispatcher().startAnimation();
	}

	@Override
	protected void stopTimer() {
		running = false;
		kernel.getApplication().getEventDispatcher().stopAnimation();
	}
}
