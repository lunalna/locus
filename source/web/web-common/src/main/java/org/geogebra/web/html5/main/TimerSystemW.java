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

import java.util.ArrayList;

import elemental2.dom.DomGlobal;
import elemental2.dom.FrameRequestCallback;

/**
 * Timer system for view repaints, driven by the browser's animation frames
 */
public class TimerSystemW {

	/**
	 * loops to wait before performing a repaint
	 */
	public static final int EUCLIDIAN_LOOPS = 0; // no wait, repaint every loop

	public static final int ALGEBRA_LOOPS = 5;

	public static final int SPREADSHEET_LOOPS = ALGEBRA_LOOPS;

	public static final int REPAINT_FLAG = 0;

	public static final int SLEEPING_FLAG = -1;

	private final AppW app;

	private final FrameLimiter limiter = new FrameLimiter();
	private final FrameRequestCallback frame = this::onFrame;
	private final ArrayList<Runnable> frameTasks = new ArrayList<>();

	private boolean running = false;
	private int idle;
	private boolean detached = false;

	/**
	 * Create new timer system
	 *
	 * @param app
	 *            application
	 */
	public TimerSystemW(AppW app) {
		this.app = app;
		this.idle = 0;
		ensureRunning();
	}

	private void onFrame(double timestamp) {
		if (running && !limiter.ready(timestamp)) {
			DomGlobal.requestAnimationFrame(frame);
		} else {
			tick();
		}
	}

	/**
	 * Execute one timer tick.
	 */
	protected void tick() {
		if (!running) {
			return;
		}
		runFrameTasks();
		if (!suggestRepaint()) {
			idle++;
		}
		if (idle > 30) {
			idle = 0;
			running = false;
			return;
		}
		DomGlobal.requestAnimationFrame(frame);
	}

	private void runFrameTasks() {
		if (frameTasks.isEmpty()) {
			return;
		}
		Runnable[] tasks = frameTasks.toArray(new Runnable[0]);
		frameTasks.clear();
		for (Runnable task : tasks) {
			task.run();
		}
	}

	/**
	 * Run a task once at the start of the next frame, before the views repaint.
	 *
	 * @param task
	 *            task to run
	 */
	public void runOnNextFrame(Runnable task) {
		frameTasks.add(task);
		ensureRunning();
	}

	/**
	 * suggests views to repaint
	 *
	 * @return whether at least one view needed repaint
	 */
	boolean suggestRepaint() {
		if (app == null || app.getKernel() == null) {
			return false;
		}
		return app.getKernel().notifySuggestRepaint();
	}

	/**
	 * Make sure the clock is ticking
	 */
	public void ensureRunning() {
		if (detached || running) {
			return;
		}
		running = true;
		DomGlobal.requestAnimationFrame(frame);
	}

	/**
	 * Stop timer, make sure it's not revived from `ensureTimerRunning`
	 * which may be called asynchronously
	 */
	public void detach() {
		running = false;
		detached = true;
	}
}
