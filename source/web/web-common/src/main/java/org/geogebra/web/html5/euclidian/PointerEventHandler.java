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

import java.util.HashSet;
import java.util.Locale;
import java.util.Objects;
import java.util.Set;

import org.geogebra.common.awt.GPoint2D;
import org.geogebra.common.euclidian.EuclidianConstants;
import org.geogebra.common.euclidian.event.PointerEventType;
import org.geogebra.gwtutil.NavigatorUtil;
import org.geogebra.web.html5.event.HasOffsets;
import org.geogebra.web.html5.event.PointerEvent;
import org.geogebra.web.html5.util.CopyPasteW;
import org.geogebra.web.html5.util.GlobalHandlerRegistry;
import org.gwtproject.dom.client.Element;
import org.jspecify.annotations.NonNull;
import org.jspecify.annotations.Nullable;

import elemental2.core.JsArray;
import elemental2.dom.DomGlobal;
import elemental2.dom.HTMLElement;
import jsinterop.base.Js;

/**
 * Handles pointer events in Euclidian view
 *
 * @author Zbynek
 *
 */
public class PointerEventHandler {

	private final IsEuclidianController tc;
	private static Element pointerCapture;
	private final HasOffsets off;
	/* Used for converting event coordinates to canvas coordinates.*/
	private final PointerState converter;

	private @Nullable PointerState first;
	private @Nullable PointerState second;
	private @Nullable PointerState third;
	private double lastOutId;
	private @Nullable GPoint2D lastOutCoords;
	/* Touches ignored because a pen is in use (palm rejection) */
	private final Set<Integer> rejectedTouches = new HashSet<>();
	private boolean penSeen;
	private boolean fingerTap;
	private double tapX;
	private double tapY;
	private static final double TAP_DISTANCE = 10;

	/**
	 * Mutable representation of pointer events
	 */
	private final class PointerState {
		private double x;
		private double y;
		private final double id;
		private final String type;

		private PointerState(elemental2.dom.PointerEvent e) {
			id = e.pointerId;
			type = e.pointerType;
			setCoords(e);
		}

		private PointerState(int id, String type) {
			this.id = id;
			this.type = type;
		}

		private void setCoords(elemental2.dom.PointerEvent e) {
			x = e.offsetX / off.getZoomLevel();
			y = e.offsetY / off.getZoomLevel();
			if (e.target instanceof HTMLElement element) {
				HTMLElement parentElement = Js.uncheckedCast(element.parentElement);
				if (parentElement != null && parentElement.dataset.get("transformLabel") != null) {
					double[] coords = {x, y};
					off.getCanvasCoords(x, y, coords, parentElement.dataset.get("transformLabel"));
					x = coords[0];
					y = coords[1];
				}
			}
		}
	}

	/**
	 * @param tc
	 *            euclidian controller
	 * @param off
	 *            offset provider
	 */
	public PointerEventHandler(IsEuclidianController tc, @NonNull HasOffsets off) {
		this.tc = tc;
		this.off = off;
		this.converter = new PointerState(0, "");
	}

	private void twoPointersDown(PointerState pointer1, PointerState pointer2) {
		tc.getLongTouchManager().cancelTimer();
		if (Objects.equals(pointer1.type, pointer2.type)) {
			tc.twoTouchStart(pointer1.x, pointer1.y, pointer2.x, pointer2.y);
		}
	}

	private void twoPointersMove(PointerState pointer1, PointerState pointer2) {
		if (Objects.equals(pointer1.type, pointer2.type)) {
			this.tc.twoTouchMove(pointer1.x, pointer1.y, pointer2.x, pointer2.y);
		}
	}

	private void singleDown(PointerEvent e) {
		tc.getOffsets().closePopups(e);
		tc.onPointerEventStart(e);
	}

	private static void adjust(PointerEvent event, elemental2.dom.PointerEvent nativeEvent) {
		event.setAlt(nativeEvent.altKey);
		event.setShift(nativeEvent.shiftKey);
		event.setControl(nativeEvent.ctrlKey);
		event.setButton(nativeEvent.button);
	}

	private void singleUp(PointerEvent e) {
		tc.getLongTouchManager().cancelTimer();
		tc.onPointerEventEnd(e);
	}

	private void setPointerType(elemental2.dom.PointerEvent event, boolean pointerDown) {
		tc.getOffsets().calculateEnvironment();
		tc.setDefaultEventType(getType(event), pointerDown);
	}

	private void startLongTouch(PointerState touchState) {
		if (tc.getMode() == EuclidianConstants.MODE_MOVE) {
			tc.getLongTouchManager().scheduleTimer(tc, (int) touchState.x, (int) touchState.y);
		}
	}

	private void checkMoveLongTouch() {
		if (tc.isDraggingBeyondThreshold()) {
			this.tc.getLongTouchManager().cancelTimer();
		}
	}

	/**
	 * Reset the pointers
	 */
	public void reset() {
		first = null;
		second = null;
		third = null;
	}

	private void onPointerMove(elemental2.dom.PointerEvent e, Element element) {
		if (isPenStrokeInterrupted(e)) {
			onPointerDown(e, element);
		}
		if (isFingerNavigation(first)) {
			navigateWithFingers(e);
			e.preventDefault();
			return;
		}
		if (first != null && second != null) {
			if (second.id == e.pointerId) {
				second.x = e.offsetX / off.getZoomLevel();
				second.y = e.offsetY / off.getZoomLevel();
				twoPointersMove(first, second);
			} else if (first.id == e.pointerId) {
				first.x = e.offsetX / off.getZoomLevel();
				first.y = e.offsetY / off.getZoomLevel();
			}
		} else if (match(first, e) || match(second, e) || "mouse".equals(e.pointerType)) {
			if (first != null && match(first, e)) {
				first.setCoords(e);
			}
			if (isDrawing(e)) {
				forwardCoalescedMoves(e);
			}
			this.tc.onPointerEventMove(convertEvent(e));
		}
		if (!"INPUT".equals(Js.<elemental2.dom.Element>uncheckedCast(e.target).tagName)) {
			e.preventDefault();
		}
		checkMoveLongTouch();
	}

	/*
	 * If type is "pen", move can only happen if the pen is touching the surface.
	 * In case we lost all pointers and we're getting pen movement,
	 * we assume that the last "pointerout" event was a glitch (happens on iPad).
	 * On Windows we can have hovering pen events => do not start a stroke
	 */
	private boolean isPenStrokeInterrupted(elemental2.dom.PointerEvent event) {
		return first == null
				&& second == null
				&& third == null
				&& NavigatorUtil.isiOS()
				&& lastOutId == event.pointerId
				&& "pen".equals(event.pointerType)
				&& event.buttons != 0;
	}

	private boolean isDrawing(elemental2.dom.PointerEvent e) {
		int mode = tc.getMode();
		return "pen".equals(e.pointerType)
				&& (mode == EuclidianConstants.MODE_PEN || mode == EuclidianConstants.MODE_HIGHLIGHTER)
				&& Js.asPropertyMap(e).has("getCoalescedEvents");
	}

	/* The pen samples faster than pointermove fires, the skipped samples make strokes smoother */
	private void forwardCoalescedMoves(elemental2.dom.PointerEvent e) {
		JsArray<elemental2.dom.PointerEvent> coalesced = e.getCoalescedEvents();
		converter.setCoords(e);
		double x = converter.x;
		double y = converter.y;
		double zoom = off.getZoomLevel();
		for (int i = 0; i < coalesced.length - 1; i++) {
			elemental2.dom.PointerEvent sample = coalesced.getAt(i);
			tc.onPointerEventMove(convertWithCoords(e, new GPoint2D(
					x + (sample.clientX - e.clientX) / zoom, y + (sample.clientY - e.clientY) / zoom)));
		}
	}

	/* Once a pen was used on the whiteboard, fingers only pan and zoom */
	private boolean isFingerNavigation(@Nullable PointerState pointer) {
		return penSeen && isType(pointer, "touch")
				&& tc.getApplication().isWhiteboardActive();
	}

	private void navigateWithFingers(elemental2.dom.PointerEvent e) {
		PointerState moved = match(first, e) ? first : match(second, e) ? second : null;
		if (moved == null || first == null) {
			return;
		}
		PointerState other = second;
		double oldX = centerX(first, other);
		double oldY = centerY(first, other);
		double oldDistance = other == null ? 0 : distance(first, other);
		moved.setCoords(e);
		if (Math.hypot(first.x - tapX, first.y - tapY) > TAP_DISTANCE) {
			fingerTap = false;
		}
		double x = centerX(first, other);
		double y = centerY(first, other);
		tc.wrapTrackpadScroll(oldX - x, oldY - y);
		if (other != null && oldDistance > 0) {
			tc.wrapPinchZoom((int) x, (int) y, distance(first, other) / oldDistance);
		}
	}

	private static double centerX(PointerState p1, @Nullable PointerState p2) {
		return p2 == null ? p1.x : (p1.x + p2.x) / 2;
	}

	private static double centerY(PointerState p1, @Nullable PointerState p2) {
		return p2 == null ? p1.y : (p1.y + p2.y) / 2;
	}

	private static double distance(PointerState p1, PointerState p2) {
		return Math.hypot(p1.x - p2.x, p1.y - p2.y);
	}

	/* Removes a navigating finger; the remaining finger keeps panning */
	private boolean releaseFinger(elemental2.dom.PointerEvent event) {
		if (!penSeen || !"touch".equals(event.pointerType)
				|| !tc.getApplication().isWhiteboardActive()) {
			return false;
		}
		if (match(first, event)) {
			first = second;
			second = null;
		} else if (match(second, event)) {
			second = null;
		}
		if (first == null) {
			setCapture(null);
			if (fingerTap && EuclidianConstants.isMoveOrSelectionMode(tc.getMode())) {
				singleDown(new PointerEvent(tapX, tapY, PointerEventType.TOUCH, off));
				singleUp(new PointerEvent(tapX, tapY, PointerEventType.TOUCH, off));
			}
			fingerTap = false;
		}
		return true;
	}

	private boolean isDown(String type) {
		return isType(first, type) || isType(second, type) || isType(third, type);
	}

	private static boolean isType(@Nullable PointerState pointer, String type) {
		return pointer != null && type.equals(pointer.type);
	}

	/* A palm that touched before the pen ends its own interaction and is ignored from now on */
	private void rejectTouches() {
		for (PointerState pointer : new PointerState[] {first, second, third}) {
			if (isType(pointer, "touch")) {
				rejectedTouches.add((int) pointer.id);
			}
		}
		PointerState touch = first;
		if (touch != null && "touch".equals(touch.type) && !isFingerNavigation(touch)) {
			singleUp(new PointerEvent(touch.x, touch.y, PointerEventType.TOUCH, off));
		}
		reset();
	}

	private boolean match(PointerState pointerState, elemental2.dom.PointerEvent event) {
		return pointerState != null && pointerState.id == event.pointerId;
	}

	private void onPointerDown(elemental2.dom.PointerEvent e, Element element) {
		if ("touch".equals(e.pointerType) && isDown("pen")) {
			rejectedTouches.add(e.pointerId);
			return;
		}
		if ("pen".equals(e.pointerType)) {
			penSeen = true;
			if (isDown("touch")) {
				rejectTouches();
			}
		}
		if (first != null && second != null && third != null) {
			reset();
			return;
		}
		lastOutCoords = null;
		setCapture(element);
		// APPS-5639 In mobile Safari copy only works from pointerup: collect now & run later
		CopyPasteW.startCollectingCopyCalls();
		if (first != null && second != null) {
			third = new PointerState(e);
			return;
		} else if (first != null) {
			second = new PointerState(e);
		} else {
			first = new PointerState(e);
		}
		if (isFingerNavigation(first)) {
			fingerTap = second == null;
			tapX = first.x;
			tapY = first.y;
			e.preventDefault();
			// down = false: keep the current tool instead of switching to select
			tc.setDefaultEventType(PointerEventType.TOUCH, false);
			return;
		}
		// prevent touch but not mouse: make sure focus is moved
		if (!"mouse".equals(e.pointerType)) {
			e.preventDefault();
		}
		setPointerType(e, true);
		if (first != null && second != null) {
			twoPointersDown(first, second);
		} else {
			singleDown(convertEvent(e));
			if ("touch".equals(e.pointerType) && first != null) {
				startLongTouch(first);
			}
		}
	}

	private PointerEvent convertEvent(elemental2.dom.PointerEvent e) {
		converter.setCoords(e);
		PointerEvent ex = new PointerEvent(converter.x, converter.y, getType(e), off);
		adjust(ex, e);
		return ex;
	}

	private PointerEvent convertWithCoords(elemental2.dom.PointerEvent e, GPoint2D coords) {
		PointerEvent ex = new PointerEvent(coords.x, coords.y, getType(e), off);
		adjust(ex, e);
		return ex;
	}

	private void onPointerUp(elemental2.dom.PointerEvent event, Element element, boolean isLocal) {
		if (pointerCapture != element || rejectedTouches.contains(event.pointerId)
				|| releaseFinger(event)) {
			return;
		}
		event.stopPropagation();
		resetPointer(event);
		if (second == null && first == null) {
			setCapture(null);
		}
		if (match(third, event)) {
			return;
		}
		if (isLocal) {
			singleUp(convertEvent(event));
		} else if (lastOutCoords != null) {
			singleUp(convertWithCoords(event, lastOutCoords));
		}
		setPointerType(event, false);
		CopyPasteW.stopCollectingCopyCalls();
	}

	private void onPointerOut(elemental2.dom.PointerEvent event) {
		// pointerout follows pointerup and pointercancel, so the touch is gone for good
		if (rejectedTouches.remove(event.pointerId) || releaseFinger(event)) {
			return;
		}
		lastOutId = event.pointerId;
		converter.setCoords(event);
		lastOutCoords = new GPoint2D(converter.x, converter.y);
		resetPointer(event);
		setPointerType(event, false);
	}

	private void resetPointer(elemental2.dom.PointerEvent event) {
		if (match(first, event)) {
			first = null;
		} else if (match(second, event)) {
			second = null;
		} else if (match(third, event)) {
			third = null;
		} else {
			reset();
		}
	}

	/**
	 * @param element listening element (EV)
	 * @param globalHandlers global registrations
	 */
	public void attachTo(Element element, GlobalHandlerRegistry globalHandlers) {
		reset();
		// treat as global to avoid memory leak
		globalHandlers.addEventListener(
				element, "pointermove", evt -> onPointerMove(Js.uncheckedCast(evt), element));

		globalHandlers.addEventListener(
				element, "pointerdown", evt -> onPointerDown(Js.uncheckedCast(evt), element));

		globalHandlers.addEventListener(
				element, "pointerout", evt -> onPointerOut(Js.uncheckedCast(evt)));

		globalHandlers.addEventListener(element, "pointercancel", evt -> {
			elemental2.dom.PointerEvent event = Js.uncheckedCast(evt);
			if (rejectedTouches.contains(event.pointerId)) {
				return;
			}
			onPointerOut(event);
			tc.onPointerCancel();
		});

		// if pointer was released in the applet, process event coordinates like "pointerdown"
		globalHandlers.addEventListener(
				element, "pointerup", evt -> onPointerUp(Js.uncheckedCast(evt), element, true));
		// if pointer was released outside, use coordinates of the last pointer leaving the applet
		// stopPropagation makes sure only one "pointerup" handler runs
		globalHandlers.addEventListener(
				DomGlobal.window, "pointerup", evt -> onPointerUp(Js.uncheckedCast(evt), element, false));
	}

	/**
	 * Make the view capture all pointer events.
	 * @param view graphics view
	 */
	public static void startCapture(EuclidianViewW view) {
		setCapture(view.getPointerTarget());
	}

	private static void setCapture(Element element) {
		pointerCapture = element;
	}

	/**
	 * @param event pointer event
	 * @return event type
	 */
	public static PointerEventType getType(elemental2.dom.PointerEvent event) {
		try {
			return PointerEventType.valueOf(event.pointerType.toUpperCase(Locale.US));
		} catch (Exception ignored) {
			// no logging: too noisy
		}
		return PointerEventType.MOUSE;
	}
}
