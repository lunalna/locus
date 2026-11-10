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

package org.geogebra.web.resources;

import java.util.HashMap;
import java.util.Map;

import org.geogebra.common.awt.ThemeColors;
import org.geogebra.gwtutil.DOMParser;
import org.geogebra.gwtutil.JsObject;
import org.geogebra.gwtutil.XMLSerializer;
import org.gwtproject.safehtml.shared.SafeUri;
import org.gwtproject.safehtml.shared.UriUtils;

import elemental2.core.Global;
import elemental2.dom.CSSStyleDeclaration;
import elemental2.dom.Document;
import elemental2.dom.DomGlobal;
import elemental2.dom.Element;
import elemental2.dom.HTMLElement;
import elemental2.dom.MutationObserver;
import elemental2.dom.MutationObserverInit;
import elemental2.dom.MutationRecord;
import elemental2.dom.Node;
import elemental2.dom.NodeList;
import jsinterop.base.Js;
import jsinterop.base.JsPropertyMap;

/**
 * Implementation of SVGResource.
 */
public class SVGResourcePrototype implements SVGResource {

	public static final SVGResource EMPTY = new SVGResourcePrototype(
			"empty", "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>");
	private static final DOMParser parser = new DOMParser();
	private static final XMLSerializer serializer = new XMLSerializer();
	// original content of every data URL, so images on the page can follow a theme change
	private static final Map<String, Original> originals = new HashMap<>();
	private static int themeGeneration;
	private static MutationObserver observer;

	private String svg;
	private String name;
	private String url;
	private int urlGeneration;

	private static final class Original {
		private final String svg;
		private final int generation;

		private Original(String svg, int generation) {
			this.svg = svg;
			this.generation = generation;
		}
	}

	/**
	 * Creates a new SVG Resource.
	 *
	 * @param svg content
	 * @param name name
	 */
	public SVGResourcePrototype(String name, String svg) {
		this.name = name;
		this.svg = svg;
	}

	/**
	 * Creates a new SVG Resource, using the resource pack's version of the file if it has one.
	 *
	 * @param name name
	 * @param path classpath path of the bundled file
	 * @param svg bundled content
	 */
	public SVGResourcePrototype(String name, String path, String svg) {
		this(name, svg);
		JsPropertyMap<String> icons = Js.uncheckedCast(
				JsObject.of(DomGlobal.window).get("resourcePackIcons"));
		if (icons != null && icons.has(path)) {
			this.svg = icons.get(path);
		}
	}

	@Override
	public SafeUri getSafeUri() {
		return UriUtils.fromTrustedString(getUrl());
	}

	@Override
	public String getName() {
		return name;
	}

	@Override
	public SVGResource withFill(String color) {
		String filled = createFilled(color, svg);
		return new SVGResourcePrototype(name, filled);
	}

	/**
	 * @param color CSS color
	 * @param svg SVG content
	 * @return filled SVG
	 */
	public static String createFilled(String color, String svg) {
		Document doc = parser.parseFromString(svg, "image/svg+xml");
		CSSStyleDeclaration style = Js.uncheckedCast(JsObject.of(doc).nestedGet("rootElement.style"));
		if (style != null) {
			style.setProperty("fill", color);
			// stroke based icons draw with currentColor
			style.setProperty("color", color);
		}
		return serializer.serializeToString(doc);
	}

	@Override
	public String getSVG() {
		return svg;
	}

	/**
	 * @return data URL of this image
	 */
	public String getUrl() {
		if (url == null || urlGeneration != themeGeneration) {
			url = toUrl(svg);
			urlGeneration = themeGeneration;
		}
		return url;
	}

	private static String toUrl(String svg) {
		String content = ThemeColors.isActive() ? getThemedSvg(svg) : svg;
		String url = "data:image/svg+xml;base64,"
				+ DomGlobal.btoa(Global.unescape(Global.encodeURIComponent(content)));
		originals.put(url, new Original(svg, themeGeneration));
		return url;
	}

	// the page's stylesheets don't reach into images, so their colors are mapped here
	private static String getThemedSvg(String svg) {
		Document doc = parser.parseFromString(svg, "image/svg+xml");
		Element root = doc.documentElement;
		// unset fill and color default to black
		for (String property : new String[] {"fill", "color"}) {
			if (!root.hasAttribute(property)) {
				root.setAttribute(property, "black");
			}
		}
		String content = serializer.serializeToString(doc);
		return content.contains("base64,") ? content : ThemeColors.mapText(content);
	}

	/**
	 * Updates the images on the page after the theme changed, including ones that were built
	 * earlier and are added later.
	 */
	public static void updateTheme() {
		themeGeneration++;
		updateTree(DomGlobal.document.body);
		if (observer == null) {
			observer = new MutationObserver((records, ignore) -> {
				for (MutationRecord record : records.asArray(new MutationRecord[0])) {
					if (record.addedNodes.length > 0) {
						for (int i = 0; i < record.addedNodes.length; i++) {
							Node node = record.addedNodes.getAt(i);
							if (node.nodeType == Node.ELEMENT_NODE) {
								updateTree(Js.uncheckedCast(node));
							}
						}
					} else {
						update(Js.uncheckedCast(record.target));
					}
				}
				return null;
			});
			MutationObserverInit init = MutationObserverInit.create();
			init.setChildList(true);
			init.setSubtree(true);
			init.setAttributes(true);
			init.setAttributeFilter(new String[] {"src", "style"});
			observer.observe(DomGlobal.document.body, init);
		}
	}

	private static void updateTree(Element root) {
		update(root);
		NodeList<Element> elements = root.querySelectorAll("img, [style*='data:image/svg']");
		for (int i = 0; i < elements.length; i++) {
			update(elements.getAt(i));
		}
	}

	private static void update(Element element) {
		if ("IMG".equals(element.tagName)) {
			String url = currentUrl(element.getAttribute("src"));
			if (url != null) {
				element.setAttribute("src", url);
			}
		}
		HTMLElement styled = Js.uncheckedCast(element);
		String background = styled.style == null ? "" : styled.style.backgroundImage;
		if (background != null && background.startsWith("url(\"data:image/svg")) {
			String url = currentUrl(background.substring(5, background.length() - 2));
			if (url != null) {
				styled.style.backgroundImage = "url(\"" + url + "\")";
			}
		}
	}

	// the URL for the current theme, or null if the given one is current or unknown
	private static String currentUrl(String url) {
		Original original = url == null ? null : originals.get(url);
		return original == null || original.generation == themeGeneration
				? null : toUrl(original.svg);
	}
}
