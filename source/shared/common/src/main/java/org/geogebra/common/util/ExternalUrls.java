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

package org.geogebra.common.util;

import java.util.HashMap;
import java.util.Map;

import org.jspecify.annotations.Nullable;

/**
 * Links and online services, supplied by the active resource pack. A missing URL turns the
 * feature off.
 */
public final class ExternalUrls {
	public static final String MANUAL = "manual";
	public static final String TUTORIALS = "tutorials";
	public static final String FORUM = "forum";
	public static final String REPORT_BUG = "reportBug";
	public static final String PRIVACY = "privacy";
	public static final String LICENSE = "license";
	public static final String WEBSITE = "website";
	public static final String MATERIALS_API = "materialsApi";
	public static final String LOGIN_API = "loginApi";
	public static final String DEPLOY_SCRIPT = "deployScript";

	private static final Map<String, String> urls = new HashMap<>();

	private ExternalUrls() {
	}

	/**
	 * @param key one of the constants
	 * @param url URL for the key
	 */
	public static void set(String key, String url) {
		urls.put(key, url);
	}

	/**
	 * @param key one of the constants
	 * @return URL, or null when the pack has none
	 */
	public static @Nullable String get(String key) {
		return urls.get(key);
	}

	/**
	 * @param key one of the constants
	 * @return whether the pack has a URL for the key
	 */
	public static boolean has(String key) {
		return urls.containsKey(key);
	}
}
