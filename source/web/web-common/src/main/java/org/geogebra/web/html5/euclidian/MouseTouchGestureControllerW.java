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

package org.geogebra.web.html5.euclidian;

import java.util.LinkedList;

import org.geogebra.common.euclidian.DrawableND;
import org.geogebra.common.euclidian.EuclidianConstants;
import org.geogebra.common.euclidian.EuclidianController;
import org.geogebra.common.euclidian.controller.MouseTouchGestureController;
import org.geogebra.common.euclidian.draw.DrawInline;
import org.geogebra.common.euclidian.event.AbstractEvent;
import org.geogebra.common.euclidian.event.PointerEventType;
import org.geogebra.common.util.StringUtil;
import org.geogebra.common.util.debug.GeoGebraProfiler;
import org.geogebra.common.util.debug.Log;
import org.geogebra.web.html5.euclidian.profiler.FpsProfilerW;
import org.geogebra.web.html5.euclidian.profiler.drawer.DrawingEmulator;
import org.geogebra.web.html5.euclidian.profiler.drawer.DrawingRecorder;
import org.geogebra.web.html5.event.HasOffsets;
import org.geogebra.web.html5.event.PointerEvent;
import org.geogebra.web.html5.event.ZeroOffset;
import org.geogebra.web.html5.gui.util.LongTouchManager;
import org.geogebra.web.html5.gui.util.LongTouchTimer.LongTouchHandler;
import org.geogebra.web.html5.main.AppW;

import elemental2.dom.DOMRect;
import elemental2.dom.DomGlobal;
import elemental2.dom.Event;
import elemental2.dom.WheelEvent;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

public class MouseTouchGestureControllerW extends MouseTouchGestureController
		implements HasOffsets {

	private static final double PINCH_ZOOM_SPEED = 0.01;
	private static final double TRACKPAD_STICKY_MS = 200;
	private static final double WHEEL_LINE_HEIGHT = 40;

	private double lastTrackpadScroll = -TRACKPAD_STICKY_MS;
	private boolean gestureActive;
	private double gestureScale = 1;
	private int delayUntilMoveFinish = 150;

	private final LongTouchManager longTouchManager;

	private boolean dragModeMustBeSelected = false;
	private int moveCounter = 0;
	private boolean dragModeIsRightClick = false;
	private final LinkedList<PointerEvent> mousePool = new LinkedList<>();
	private final LinkedList<PointerEvent> touchPool = new LinkedList<>();
	private boolean comboboxFocused;

	private DrawingEmulator drawingEmulator;
	private DrawingRecorder drawingRecorder;
	private boolean isRecording;

	/**
	 * recalculates cached styles concerning browser environment
	 */
	public void calculateEnvironment() {
		if (ec.getView() == null) {
			return;
		}

		ec.getView().setPixelRatio(((AppW) app).getGraphicsPixelRatio());
	}

	/**
	 * Create new mouse / touch handler.
	 *
	 * @param app
	 *            application
	 * @param ec
	 *            euclidian controller
	 */
	public MouseTouchGestureControllerW(AppW app, EuclidianController ec) {
		super(app, ec);
		this.app = app;

		app.getGlobalHandlers()
				.addEventListener(DomGlobal.window, "scroll", e -> calculateEnvironment());
		app.addWindowResizeListener(this::calculateEnvironment);
		longTouchManager = LongTouchManager.getInstance();
	}

	/**
	 * Fire touch event at the specified coordinates (right-click if not unbundled)
	 */
	public void handleLongTouch(double x, double y) {
		PointerEvent event = new PointerEvent(x, y, PointerEventType.TOUCH, ZeroOffset.INSTANCE);
		if (!app.isUnbundled()) {
			event.setIsRightClick(true);
		}
		ec.wrapMouseReleased(event);
	}

	/**
	 * Handle mouse scroll.
	 *
	 * @param event
	 *            mouse wheel event
	 */
	public void onMouseWheel(WheelEvent event) {
		// don't want to roll the scrollbar
		double delta = event.deltaY;
		// we are on device where many small scrolls come, we want to merge them
		int x = (int) Math.round(event.offsetX / getZoomLevel());
		int y = (int) Math.round(event.offsetY / getZoomLevel());
		boolean shiftOrMeta = event.shiftKey || event.metaKey;
		boolean consumed = false;
		if (!ec.getView().isEuclidianView3D() && event.deltaMode == 0) {
			if (event.ctrlKey && gestureActive) {
				consumed = true;
			} else if (event.ctrlKey) {
				// trackpad pinch in Chromium / Firefox; clamp so ctrl + mouse wheel stays sane
				double pinch = Math.max(-10, Math.min(10, delta));
				consumed = ec.wrapPinchZoom(x, y, Math.exp(-pinch * PINCH_ZOOM_SPEED));
			} else if (isTrackpadScroll(event)) {
				consumed = ec.wrapTrackpadScroll(event.deltaX, delta);
			}
			if (consumed) {
				event.preventDefault();
				return;
			}
		}
		// the whiteboard scrolls with the wheel and zooms with ctrl / cmd + wheel
		if (app.isWhiteboardActive() && !event.ctrlKey && !event.metaKey) {
			double unit = event.deltaMode == 0 ? 1 : WHEEL_LINE_HEIGHT;
			double dx = (event.shiftKey ? delta : event.deltaX) * unit;
			double dy = (event.shiftKey ? 0 : delta) * unit;
			if (ec.wrapTrackpadScroll(dx, dy)) {
				event.preventDefault();
			}
			return;
		}
		if (delta != 0) {
			consumed = ec.wrapMouseWheelMoved(x, y, delta, shiftOrMeta, event.altKey);
		}
		if (consumed || ec.allowMouseWheel(shiftOrMeta)) {
			event.preventDefault();
		}
	}

	private boolean isTrackpadScroll(WheelEvent event) {
		double now = DomGlobal.performance.now();
		double wheelDeltaY = Js.asPropertyMap(event).getAsAny("wheelDeltaY").asDouble();
		// mouse wheels report wheelDelta in steps of 120, trackpads report -3 * deltaY
		boolean trackpad = event.deltaX != 0
				|| (wheelDeltaY == -3 * event.deltaY && wheelDeltaY % 120 != 0)
				|| now - lastTrackpadScroll < TRACKPAD_STICKY_MS;
		if (trackpad) {
			lastTrackpadScroll = now;
		}
		return trackpad;
	}

	/**
	 * Handle WebKit pinch gesture (gesturestart / gesturechange / gestureend).
	 *
	 * @param event
	 *            gesture event
	 */
	public void onGesture(Event event) {
		event.preventDefault();
		JsPropertyMap<Object> props = Js.asPropertyMap(event);
		double scale = props.getAsAny("scale").asDouble();
		switch (event.type) {
			case "gesturestart":
				gestureActive = true;
				break;
			case "gesturechange":
				// iOS also sends these for touch pinch, which the touch handlers already zoom
				if (ec.getDefaultEventType() == PointerEventType.TOUCH) {
					break;
				}
				elemental2.dom.Element target = Js.uncheckedCast(event.currentTarget);
				DOMRect rect = target.getBoundingClientRect();
				int x = (int) Math.round(
						(props.getAsAny("clientX").asDouble() - rect.left) / getZoomLevel());
				int y = (int) Math.round(
						(props.getAsAny("clientY").asDouble() - rect.top) / getZoomLevel());
				ec.wrapPinchZoom(x, y, scale / gestureScale);
				break;
			default:
				gestureActive = false;
		}
		gestureScale = scale;
	}

	/**
	 * Handle mouse move event immediately.
	 *
	 * @param event
	 *            touch move
	 * @param time
	 *            current time
	 * @param startCapture
	 *            whether to start capturing
	 */
	public void onMouseMoveNow(PointerEvent event, long time, boolean startCapture) {
		if (!dragModeMustBeSelected) {
			ec.wrapMouseMoved(event);
		} else {
			event.setIsRightClick(dragModeIsRightClick);
			wrapMouseDraggedWithProfiling(event, startCapture);
			if (isRecording) {
				drawingRecorder.recordCoordinate(event.getX(), event.getY(), System.currentTimeMillis());
			}
		}
		event.release();
		int dragTime = (int) (System.currentTimeMillis() - time);
		if (dragTime > delayUntilMoveFinish) {
			delayUntilMoveFinish = dragTime + 10;
		}

		moveCounter++;
	}

	private void wrapMouseDraggedWithProfiling(PointerEvent event, boolean startCapture) {
		double dragStart = FpsProfilerW.getMillisecondTimeNative();
		ec.wrapMouseDragged(event, startCapture);
		GeoGebraProfiler.addDrag((long) (FpsProfilerW.getMillisecondTimeNative() - dragStart));
	}

	/**
	 * Handle touch end or mouse up.
	 *
	 * @param e
	 *            pointer up event
	 */
	public void onPointerEventEnd(AbstractEvent e) {
		app.getFpsProfiler().notifyTouchEnd();
		if (isRecording) {
			drawingRecorder.recordTouchEnd();
		}
		if (moveCounter < 2) {
			ec.resetModeAfterFreehand();
		}
		resetDelay();
		dragModeMustBeSelected = false;

		// hide dialogs if they are open
		// but don't hide context menu if we just opened it via long tap in IE
		if (ec.getDefaultEventType() == PointerEventType.MOUSE && app.getGuiManager() != null) {
			((AppW) app).getGuiManager().removePopup();
		}

		ec.wrapMouseReleased(e);
		e.release();

		ec.resetModeAfterFreehand();
	}

	/**
	 * Handle pointer start event.
	 *
	 * @param event
	 *            pointer start
	 */
	public void onPointerEventStart(AbstractEvent event) {
		app.getFpsProfiler().notifyTouchStart();
		app.hideMenu();

		if (isRecording) {
			drawingRecorder.recordCoordinate(event.getX(), event.getY(), System.currentTimeMillis());
		}
		if (event.getType() == PointerEventType.TOUCH
				&& EuclidianConstants.isMoveOrSelectionMode(ec.getMode())) {
			longTouchManager.scheduleTimer((LongTouchHandler) ec, event.getX(), event.getY());
		}

		if (!ec.isTextfieldHasFocus()) {
			dragModeMustBeSelected = true;
			dragModeIsRightClick = event.isRightClick();
		}

		ec.wrapMousePressed(event);
		// hide PopUp if no hits was found.
		if (ec.getView().getHits().isEmpty() && ec.getView().hasStyleBar()) {
			ec.getView().getStyleBar().hidePopups();
		}
		if (!event.isRightClick()) {
			ec.prepareModeForFreehand();
		}
		event.release();
	}

	/**
	 * TODO remove?
	 */
	public void resetToolTipManager() {
		// not implemented
	}

	@Override
	public LinkedList<PointerEvent> getMouseEventPool() {
		return mousePool;
	}

	@Override
	public LinkedList<PointerEvent> getTouchEventPool() {
		return touchPool;
	}

	public boolean isComboboxFocused() {
		return this.comboboxFocused;
	}

	public void setComboboxFocused(boolean flag) {
		this.comboboxFocused = flag;
	}

	@Override
	public int getEvID() {
		return ec.getView().getViewID();
	}

	@Override
	public PointerEventType getDefaultEventType() {
		return ec.getDefaultEventType();
	}

	@Override
	public double getZoomLevel() {
		String zoom =
				((AppW) app).getGeoGebraElement().getParentElement().getStyle().getProperty("zoom");
		if (StringUtil.empty(zoom)) {
			return 1;
		}
		try {
			return Double.parseDouble(zoom);
		} catch (NumberFormatException e) {
			return 1;
		}
	}

	public LongTouchManager getLongTouchManager() {
		return longTouchManager;
	}

	/**
	 * Reset move finish delay to default.
	 */
	public void resetDelay() {
		delayUntilMoveFinish = 150;
	}

	/**
	 * Close all popups.
	 */
	public void closePopups(PointerEvent e) {
		app.closePopups(e.getX(), e.getY());
	}

	/**
	 * @return drawing emulator
	 */
	public DrawingEmulator getDrawingEmulator() {
		if (drawingEmulator == null) {
			drawingEmulator = new DrawingEmulator(this);
		}
		return drawingEmulator;
	}

	/**
	 * Records the drawing.
	 */
	public void startDrawRecording() {
		isRecording = true;
		if (drawingRecorder == null) {
			drawingRecorder = new DrawingRecorder();
		}
	}

	/**
	 * Ends the recording of the drawing and logs the results.
	 *
	 * For autonomous drawing, the logged result has to be copied into the coords.json file.
	 */
	public void endDrawRecordingAndLogResult() {
		Log.debug(drawingRecorder);
		drawingRecorder.reset();
		isRecording = false;
	}

	public boolean isDragging() {
		return dragModeMustBeSelected;
	}

	@Override
	public void getCanvasCoords(double x, double y, double[] coords, String transformLabel) {
		DrawableND drawable = ec.getView().getDrawableFor(app.getKernel().lookupLabel(transformLabel));
		if (drawable instanceof DrawInline drawInline) {
			double contentScale = drawInline.getScale();
			drawInline
					.getTransform()
					.transform(new double[] {x * contentScale, y * contentScale}, 0, coords, 0, 1);
		}
	}
}
