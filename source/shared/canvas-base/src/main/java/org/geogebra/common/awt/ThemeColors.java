package org.geogebra.common.awt;

import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.function.DoubleToIntFunction;

/**
 * Display-only color mapping of the active theme. Each color gets the contrast against the theme
 * background that it had against white, keeping its hue. Files are unaffected.
 */
public final class ThemeColors {

	private static int background = -1;
	private static int foreground;
	private static int version;
	private static final Set<Integer> kept = new HashSet<>();
	private static Map<Integer, Integer> accents = new HashMap<>();
	private static final Map<Integer, Integer> cache = new HashMap<>();

	private ThemeColors() {
	}

	/**
	 * @param backgroundRgb theme background as 0xRRGGBB, -1 for none
	 * @param foregroundRgb theme foreground as 0xRRGGBB
	 */
	public static void set(int backgroundRgb, int foregroundRgb) {
		background = backgroundRgb;
		foreground = foregroundRgb;
		cache.clear();
		version++;
	}

	/**
	 * @return counter that changes whenever the color mapping changes
	 */
	public static int getVersion() {
		return version;
	}

	/**
	 * @param rgb accent color as 0xRRGGBB, which is used as is
	 */
	public static void keep(int rgb) {
		kept.add(rgb);
	}

	/**
	 * Replaces the accent colors after a theme change, since the color constants keep the old ones.
	 * @param replaced old accent colors mapped to the new ones as 0xRRGGBB
	 * @param keptRgb accent colors that are used as is
	 */
	public static void setAccents(Map<Integer, Integer> replaced, Set<Integer> keptRgb) {
		accents = replaced;
		kept.clear();
		kept.addAll(keptRgb);
		cache.clear();
		version++;
	}

	/**
	 * @return whether a theme maps colors
	 */
	public static boolean isActive() {
		return background >= 0 || !accents.isEmpty();
	}

	/**
	 * @param rgb color as 0xRRGGBB
	 * @return mapped color as 0xRRGGBB
	 */
	public static int map(int rgb) {
		Integer accent = accents.get(rgb);
		if (accent != null) {
			return accent;
		}
		if (kept.contains(rgb) || background < 0) {
			return rgb;
		}
		Integer mapped = cache.get(rgb);
		if (mapped == null) {
			mapped = matchContrast(rgb);
			cache.put(rgb, mapped);
		}
		return mapped;
	}

	private static int matchContrast(int rgb) {
		double contrast = 1.05 / (luminance(rgb) + 0.05);
		double base = luminance(background) + 0.05;
		double target = luminance(background) < luminance(foreground)
				? Math.min(1, base * contrast - 0.05) : Math.max(0, base / contrast - 0.05);
		double[] hsl = hsl(rgb);
		// grays follow the theme's own tint, saturated colors keep their hue
		int gray = withLuminance(t -> mix(background, foreground, t), target);
		int colored = withLuminance(l -> fromHsl(hsl[0], hsl[1], l), target);
		return mix(gray, colored, hsl[1]);
	}

	// binary search on a color scale whose luminance changes monotonically
	private static int withLuminance(DoubleToIntFunction scale, double target) {
		boolean increasing = luminance(scale.applyAsInt(1)) > luminance(scale.applyAsInt(0));
		double low = 0;
		double high = 1;
		for (int i = 0; i < 20; i++) {
			double middle = (low + high) / 2;
			if (luminance(scale.applyAsInt(middle)) < target == increasing) {
				low = middle;
			} else {
				high = middle;
			}
		}
		return scale.applyAsInt((low + high) / 2);
	}

	private static double luminance(int rgb) {
		int[] c = channels(rgb);
		return 0.2126 * linear(c[0]) + 0.7152 * linear(c[1]) + 0.0722 * linear(c[2]);
	}

	private static double linear(int channel) {
		double c = channel / 255.0;
		return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
	}

	private static int mix(int from, int to, double t) {
		int[] a = channels(from);
		int[] b = channels(to);
		int result = 0;
		for (int i = 0; i < 3; i++) {
			result = (result << 8) + (int) Math.round(a[i] + (b[i] - a[i]) * t);
		}
		return result;
	}

	private static double[] hsl(int rgb) {
		int[] c = channels(rgb);
		int max = Math.max(c[0], Math.max(c[1], c[2]));
		int min = Math.min(c[0], Math.min(c[1], c[2]));
		double l = (max + min) / 510.0;
		if (max == min) {
			return new double[] {0, 0, l};
		}
		double d = (max - min) / 255.0;
		double s = d / (1 - Math.abs(2 * l - 1));
		double r = c[0] / 255.0;
		double g = c[1] / 255.0;
		double b = c[2] / 255.0;
		double h;
		if (max == c[0]) {
			h = ((g - b) / d + 6) % 6;
		} else if (max == c[1]) {
			h = (b - r) / d + 2;
		} else {
			h = (r - g) / d + 4;
		}
		return new double[] {h / 6, s, l};
	}

	private static int fromHsl(double h, double s, double l) {
		double chroma = (1 - Math.abs(2 * l - 1)) * s;
		double[] rgb = new double[3];
		for (int i = 0; i < 3; i++) {
			// standard HSL formula, evaluated for red, green and blue
			double k = (new double[] {0, 8, 4}[i] + h * 12) % 12;
			rgb[i] = l - chroma / 2 * Math.max(-1, Math.min(Math.min(k - 3, 9 - k), 1));
		}
		return (int) Math.round(rgb[0] * 255) << 16 | (int) Math.round(rgb[1] * 255) << 8
				| (int) Math.round(rgb[2] * 255);
	}

	/**
	 * @param color color
	 * @return mapped color with the same alpha
	 */
	public static GColor map(GColor color) {
		int rgb = map(color.getARGB() & 0xFFFFFF);
		return GColor.newColor(rgb >> 16, (rgb >> 8) & 0xFF, rgb & 0xFF, color.getAlpha());
	}

	/**
	 * @param color color
	 * @return the color as shown on screen
	 */
	public static GColor forDisplay(GColor color) {
		return isActive() ? map(color) : color;
	}

	/**
	 * @param color CSS color
	 * @return the CSS color as shown on screen
	 */
	public static String css(String color) {
		return isActive() ? mapText(color) : color;
	}

	/**
	 * Maps hex, rgb(), rgba(), white and black colors in CSS or SVG text.
	 * @param text CSS value or SVG markup
	 * @return text with mapped colors
	 */
	public static String mapText(String text) {
		StringBuilder out = new StringBuilder();
		int i = 0;
		while (i < text.length()) {
			int end = i;
			String replacement = null;
			char ch = text.charAt(i);
			if (ch == '#' && !isWordChar(text, i - 1)) {
				end = i + 1;
				while (end < text.length() && Character.digit(text.charAt(end), 16) >= 0) {
					end++;
				}
				// url(#id) and #id-like references are not colors
				boolean reference = i > 0 && text.charAt(i - 1) == '(' || isWordChar(text, end);
				replacement = reference ? null : mapHex(text.substring(i + 1, end));
			} else if (text.startsWith("rgb", i) && !isWordChar(text, i - 1)) {
				int open = text.indexOf('(', i);
				int close = text.indexOf(')', i);
				if (open > 0 && open <= i + 4 && close > open) {
					end = close + 1;
					replacement = mapRgb(text.substring(open + 1, close));
				}
			} else if (!isWordChar(text, i - 1)) {
				for (String name : new String[] {"white", "black"}) {
					if (text.startsWith(name, i) && !isWordChar(text, i + name.length())) {
						end = i + name.length();
						replacement = toHex(map("white".equals(name) ? 0xFFFFFF : 0));
					}
				}
			}
			if (replacement == null) {
				out.append(ch);
				i++;
			} else {
				out.append(replacement);
				i = end;
			}
		}
		return out.toString();
	}

	private static String mapHex(String digits) {
		String hex = digits;
		if (hex.length() == 3 || hex.length() == 4) {
			StringBuilder full = new StringBuilder();
			for (char digit : hex.toCharArray()) {
				full.append(digit).append(digit);
			}
			hex = full.toString();
		}
		if (hex.length() != 6 && hex.length() != 8) {
			return null;
		}
		return toHex(map(Integer.parseInt(hex.substring(0, 6), 16))) + hex.substring(6);
	}

	private static String mapRgb(String args) {
		String[] parts = args.split("[,/\\s]+");
		try {
			int rgb = 0;
			for (int i = 0; i < 3; i++) {
				rgb = (rgb << 8) + Math.round(Float.parseFloat(parts[i].trim()));
			}
			int mapped = map(rgb);
			String channels = (mapped >> 16) + "," + ((mapped >> 8) & 0xFF) + "," + (mapped & 0xFF);
			return parts.length > 3 ? "rgba(" + channels + "," + parts[3].trim() + ")"
					: "rgb(" + channels + ")";
		} catch (RuntimeException e) {
			// percentages and other forms stay as they are
			return null;
		}
	}

	private static boolean isWordChar(String text, int index) {
		if (index < 0 || index >= text.length()) {
			return false;
		}
		char c = text.charAt(index);
		return Character.isLetterOrDigit(c) || c == '-' || c == '_';
	}

	private static String toHex(int rgb) {
		String hex = Integer.toHexString(rgb | 0x1000000);
		return "#" + hex.substring(1);
	}

	private static int[] channels(int rgb) {
		return new int[] {(rgb >> 16) & 0xFF, (rgb >> 8) & 0xFF, rgb & 0xFF};
	}
}
