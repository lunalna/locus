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

package org.geogebra.common.euclidian;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

import org.geogebra.common.awt.AwtFactory;
import org.geogebra.common.awt.GAffineTransform;
import org.geogebra.common.awt.GGeneralPath;
import org.geogebra.common.awt.GGraphics2D;
import org.geogebra.common.awt.GPathIterator;
import org.geogebra.common.awt.GPoint2D;
import org.geogebra.common.awt.GRectangle;
import org.geogebra.common.awt.GRectangle2D;
import org.geogebra.common.awt.GShape;
import org.geogebra.common.kernel.MyPoint;
import org.geogebra.common.kernel.SegmentType;
import org.geogebra.common.util.MyMath;

/**
 * A GeneralPath implementation that does clipping of line segments at the
 * screen in double coordinates. This is important to avoid rendering problems
 * that occur with GeneralPath when coordinates are larger than Float.MAX_VALUE.
 *
 * @author Markus Hohenwarter
 * @version October 2009
 */
public class GeneralPathClipped {
	private static final double EPSILON = 0.01;
	// unprocessed path points, a null type marks an unused position
	private double[] xs = new double[16];
	private double[] ys = new double[16];
	private SegmentType[] types = new SegmentType[16];
	private int count;
	/** Cached clipped path, considered invalid if there are unprocessed points */
	private final GGeneralPath gp;
	// end of the last segment added to gp
	private boolean hasCurrent;
	private double currentX;
	private double currentY;

	private static final double MAX_COORD_VALUE = 10000;

	/** view */
	protected EuclidianViewInterfaceSlim view;

	private int lineThickness;

	private boolean smallPolygon = true;
	private boolean polygon = true;

	private boolean needClosePath;
	private GRectangle2D bounds;
	private boolean hasBounds;
	private double minX;
	private double minY;
	private double maxX;
	private double maxY;
	private double auxX;
	private double auxY;
	// first control point
	private double cont1X = Double.NaN;
	private double cont1Y = Double.NaN;
	// second control point
	private double cont2X = Double.NaN;
	private double cont2Y = Double.NaN;
	private final ClipAlgoSutherlandHodogman clipAlgoSutherlandHodogman;

	public GeneralPathClipped(EuclidianViewInterfaceSlim view) {
		this(view, GPathIterator.WIND_EVEN_ODD);
	}

	/**
	 * Creates new clipped general path
	 *
	 * @param view
	 *            view
	 */
	public GeneralPathClipped(EuclidianViewInterfaceSlim view, int rule) {
		this.view = view;
		clipAlgoSutherlandHodogman = new ClipAlgoSutherlandHodogman();
		gp = AwtFactory.getPrototype().newGeneralPath(rule);
	}

	/**
	 * @return x-coord of the first point, only valid if size() &gt; 0
	 */
	public final double getFirstX() {
		return xs[0];
	}

	/**
	 * @return y-coord of the first point, only valid if size() &gt; 0
	 */
	public final double getFirstY() {
		return ys[0];
	}

	/**
	 * @apiNote This should not be called after {@code getGeneralPath}.
	 * @return x-coord of the last point, only valid if size() &gt; 0
	 */
	public final double getCurrentX() {
		return xs[count - 1];
	}

	/**
	 * @apiNote This should not be called after {@code getGeneralPath}.
	 * @return y-coord of the last point, only valid if size() &gt; 0
	 */
	public final double getCurrentY() {
		return ys[count - 1];
	}

	/**
	 * Clears all points and resets internal variables
	 */
	public final void reset() {
		clearPoints();
		gp.reset();
		hasBounds = false;
		smallPolygon = true;
		polygon = true;
		needClosePath = false;
	}

	/**
	 * Clears all points and resets internal variables
	 * and the line thickness too.
	 * @param lineThickness line thickness
	 */
	public final void resetWithThickness(int lineThickness) {
		reset();
		this.lineThickness = lineThickness;
	}

	/**
	 * Closes path
	 */
	public final void closePath() {
		needClosePath = true;
	}

	/**
	 * @return this as GeneralPath
	 */
	public GGeneralPath getGeneralPath() {
		if (count == 0) {
			return gp;
		}

		gp.reset();
		hasCurrent = false;
		if (smallPolygon || !polygon) {
			addSimpleSegments();
		} else {
			addClippedSegmentsWithSutherlandHodogman();
		}

		clearPoints();

		return gp;
	}

	private void clearPoints() {
		for (int i = 0; i < count; i++) {
			types[i] = null;
		}
		count = 0;
	}

	private void addSimpleSegments() {
		for (int i = 0; i < count; i++) {
			if (types[i] != null) {
				addToGeneralPath(xs[i], ys[i], types[i]);
			}
		}
		if (needClosePath) {
			gp.closePath();
		}
	}

	private void addClippedSegmentsWithSutherlandHodogman() {
		int padding = lineThickness + 5;
		double[][] clipPoints = {
			{-padding, -padding},
			{-padding, view.getHeight() + padding},
			{view.getWidth() + padding, view.getHeight() + padding},
			{view.getWidth() + padding, -padding},
		};

		ArrayList<MyPoint> points = new ArrayList<>(count);
		for (int i = 0; i < count; i++) {
			if (types[i] != null) {
				points.add(new MyPoint(xs[i], ys[i], types[i]));
			}
		}
		if (needClosePath && !points.isEmpty()) {
			points.get(0).setLineTo(true);
		}

		List<MyPoint> result = clipAlgoSutherlandHodogman.process(points, clipPoints);

		for (MyPoint curP : result) {
			addToGeneralPath(curP.x, curP.y, curP.getSegmentType());
		}

		if (!result.isEmpty() && needClosePath) {
			gp.closePath();
		}
	}

	private void addToGeneralPath(double x, double y, SegmentType lineTo) {
		if (lineTo == SegmentType.CONTROL) {
			if (Double.isNaN(cont1X) && Double.isNaN(cont1Y)) {
				cont1X = x;
				cont1Y = y;
			} else {
				cont2X = x;
				cont2Y = y;
			}
			return;
		} else if (lineTo == SegmentType.CURVE_TO) {
			if (!Double.isNaN(cont1X)
					&& !Double.isNaN(cont1Y)
					&& !Double.isNaN(cont2X)
					&& !Double.isNaN(cont2Y)) {
				gp.curveTo(cont1X, cont1Y, cont2X, cont2Y, x, y);
			} else if (!Double.isNaN(cont1X) && !Double.isNaN(cont1Y)) {
				gp.quadTo(cont1X, cont1Y, x, y);
			} else {
				return;
			}
			cont1X = Double.NaN;
			cont1Y = Double.NaN;
			cont2X = Double.NaN;
			cont2Y = Double.NaN;
		} else if (lineTo == SegmentType.AUXILIARY) {
			auxX = x;
			auxY = y;
			return;
		} else if (lineTo == SegmentType.ARC_TO && hasCurrent) {
			double dx1 = auxX - currentX;
			double dy1 = auxY - currentY;
			double dx2 = auxX - x;
			double dy2 = auxY - y;
			double angle = MyMath.angle(dx1, dy1, dx2, dy2);
			double cv = btan(Math.PI - angle) * Math.tan(angle / 2);
			gp.curveTo(
					currentX + dx1 * cv,
					currentY + dy1 * cv,
					x + dx2 * cv,
					y + dy2 * cv,
					x,
					y);
		} else if (lineTo == SegmentType.LINE_TO && hasCurrent) {
			// Safari: 0 length segments not drawn (MOW-1818 / MOW-878)
			if (MyMath.length(x - currentX, y - currentY) < EPSILON) {
				gp.lineTo(x + EPSILON, y);
				currentX = x + EPSILON;
				currentY = y;
				return;
			}
			gp.lineTo(x, y);
		} else {
			gp.moveTo(x, y);
		}
		hasCurrent = true;
		currentX = x;
		currentY = y;
	}

	private static double btan(double angle) {
		double increment = angle / 2.0;
		return 4.0 / 3.0 * Math.sin(increment) / (1.0 + Math.cos(increment));
	}

	/**
	 * Move to (x,y).
	 *
	 * @param x
	 *            x-coord
	 * @param y
	 *            y-coord
	 */
	public final void moveTo(double x, double y) {
		addPoint(x, y, SegmentType.MOVE_TO);
	}

	/**
	 * Line to (x,y).
	 *
	 * @param x
	 *            x-coord
	 * @param y
	 *            y-coord
	 */
	public final void lineTo(double x, double y) {
		addPoint(x, y, SegmentType.LINE_TO);
	}

	/**
	 * Adds point to point list and keeps track of largest coordinate.
	 *
	 * @param pos
	 *            insert position
	 * @param x
	 *            x-coord
	 * @param y
	 *            y-coord
	 */
	public final void addPoint(int pos, double x, double y) {
		if (Double.isNaN(y)) {
			return;
		}

		ensureCapacity(pos + 1);
		xs[pos] = x;
		ys[pos] = y;
		types[pos] = SegmentType.LINE_TO;
		count = Math.max(count, pos + 1);
	}

	private void ensureCapacity(int capacity) {
		if (capacity > xs.length) {
			int length = Math.max(capacity, 2 * xs.length);
			xs = Arrays.copyOf(xs, length);
			ys = Arrays.copyOf(ys, length);
			types = Arrays.copyOf(types, length);
		}
	}

	/**
	 * Adds point to point list and keeps track of largest coordinate.
	 *
	 * @param x
	 *            x-coord
	 * @param y
	 *            y-coord
	 * @param segmentType
	 *            path segment type
	 */
	protected void addPoint(double x, double y, SegmentType segmentType) {
		if (Double.isNaN(y)) {
			return;
		}

		if (segmentType != SegmentType.LINE_TO && segmentType != SegmentType.MOVE_TO) {
			polygon = false;
		}

		if (!hasBounds) {
			hasBounds = true;
			minX = x;
			maxX = x;
			minY = y;
			maxY = y;
		} else {
			if (x < minX) {
				minX = x;
			} else if (x > maxX) {
				maxX = x;
			}
			if (y < minY) {
				minY = y;
			} else if (y > maxY) {
				maxY = y;
			}
		}
		smallPolygon =
				smallPolygon && polygon && Math.abs(x) < MAX_COORD_VALUE && Math.abs(y) < MAX_COORD_VALUE;

		ensureCapacity(count + 1);
		xs[count] = x;
		ys[count] = y;
		types[count] = segmentType;
		count++;
	}

	/**
	 * @param p
	 *            point
	 * @return true if contains given point
	 */
	public boolean contains(GPoint2D p) {
		return getGeneralPath().contains(p);
	}

	/**
	 * @param rect
	 *            rectangle
	 * @return true if contains given rectangle
	 */
	public boolean contains(GRectangle2D rect) {
		return getGeneralPath().contains(rect);
	}

	/**
	 * @param x
	 *            x min
	 * @param y
	 *            y min
	 * @param w
	 *            width
	 * @param h
	 *            height
	 * @return true if contains rectangle given by args
	 */
	public boolean contains(double x, double y, double w, double h) {
		return getGeneralPath().contains(x, y, w, h);
	}

	/**
	 * @param x x-coord
	 * @param y y-coord
	 * @return whether area enclosed by this path contains given point
	 */
	public boolean contains(int x, int y) {
		return getGeneralPath().contains(x, y);
	}

	/**
	 * @param rectangle
	 *            rectangle to be checked
	 * @return whether rectangle is contained in this path
	 */
	public boolean contains(GRectangle rectangle) {
		return getGeneralPath().contains(rectangle);
	}

	/**
	 * @return path bounds
	 */
	public GRectangle getBounds() {
		return hasBounds ? getBounds2D().getBounds() : AwtFactory.getPrototype().newRectangle();
	}

	/**
	 * @return path bounds
	 */
	public GRectangle2D getBounds2D() {
		if (!hasBounds) {
			return AwtFactory.getPrototype().newRectangle2D();
		}
		if (bounds == null) {
			bounds = AwtFactory.getPrototype().newRectangle2D();
		}
		bounds.setRect(minX, minY, maxX - minX, maxY - minY);
		return bounds;
	}

	/**
	 * @param arg0 transform
	 * @return path iterator
	 */
	public GPathIterator getPathIterator(GAffineTransform arg0) {
		return getGeneralPath().getPathIterator(arg0);
	}

	/**
	 * @param arg0 rectangle
	 * @return whether this intersects given rectangle
	 */
	public boolean intersects(GRectangle2D arg0) {
		return getGeneralPath().intersects(arg0);
	}

	/**
	 * Checks for intersection with a rectangle.
	 * @return whether this intersects given rectangle.
	 */
	public boolean intersects(double x, double y, double w, double h) {
		return getGeneralPath().intersects(x, y, w, h);
	}

	/**
	 * @param x
	 *            center x-coord
	 * @param y
	 *            center y-coord
	 * @param radius
	 *            inradius of the square
	 * @return whether this intersects square with center (x,y) and inradius
	 *         radius
	 */
	public boolean intersects(int x, int y, int radius) {
		return getGeneralPath().intersects(x - radius, y - radius, 2 * radius, 2 * radius);
	}

	/**
	 * @return number of points
	 */
	public int size() {
		return count;
	}

	/**
	 * Append given path to this to create one continuous path (ignoring CLOSE and MOVE_TOs).
	 * NOTE: QUAD_TO is not supported.
	 * @param path path to append
	 */
	public void append(GShape path) {
		GPathIterator iterator = path.getPathIterator(null);
		double[] current = new double[6];
		while (!iterator.isDone()) {
			int type = iterator.currentSegment(current);
			iterator.next();
			switch (type) {
				case GPathIterator.SEG_LINETO:
					lineTo(current[0], current[1]);
					break;
				case GPathIterator.SEG_CUBICTO:
					addPoint(current[0], current[1], SegmentType.CONTROL);
					addPoint(current[2], current[3], SegmentType.CONTROL);
					addPoint(current[4], current[5], SegmentType.CURVE_TO);
					break;
				default: // skip
					break;
			}
		}
	}

	/**
	 * Draw this onto a graphics.
	 * @param g2 graphics
	 */
	public void draw(GGraphics2D g2) {
		g2.draw(getGeneralPath());
	}
}
