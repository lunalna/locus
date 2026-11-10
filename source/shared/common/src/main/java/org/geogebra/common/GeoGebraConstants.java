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

package org.geogebra.common;

/**
 * Application-wide constants (version strings, URLs)
 */
public interface GeoGebraConstants {

	// GeoGebra version
	// DO NOT CHANGE the format of VERSION_STRING (or add commented out version)
	// as it is read by the build system
	// and updated automatically by the "Version Bump" task
	/** last build date */
	String BUILD_DATE = "05 November 2026";
	/** complete version string */
	String VERSION_STRING = "5.4.936.0";
	/** proper noun, should NOT be translated / transliterated */
	String APPLICATION_NAME = "Locus";

	/** App versions */
	enum Version {
		CAS,
		GRAPHING,
		GRAPHING_3D,
		GEOMETRY,
		SCIENTIFIC,
		SUITE,
		MIXED_REALITY,
		NOTES,
		PROBABILITY,
		CLASSIC;

		/**
		 * @return translatable name of this app type
		 */
		public String getTransKey() {
			switch (this) {
				case GRAPHING:
					return "GeoGebraGraphingCalculator";
				case GRAPHING_3D:
					return "GeoGebra3DGrapher";
				case SCIENTIFIC:
					return "GeoGebraScientificCalculator";
				case CAS:
					return "GeoGebraCASCalculator";
				case GEOMETRY:
					return "GeoGebraGeometry";
				case NOTES:
					return "GeoGebraNotes";
				case SUITE:
					return "GeoGebraCalculatorSuite";
				default:
					return null;
			}
		}
	}

	/**
	 * Platform
	 */
	enum Platform {
		DESKTOP("d"),

		/** GeoGebra Graphing Calculator */
		ANDROID("a"),

		WEB("w"),

		IOS("i"),

		IOS_WEBVIEW("iw"),

		WEB_FOR_BROWSER_2D("w2d"),

		OFFLINE("offline"),

		SMART("smart"),

		POWERPOINT("p");

		private final String name;

		Platform(String name) {
			this.name = name;
		}

		public String getName() {
			return name;
		}

		/**
		 * @param prerelease
		 *            whether we run prerelease
		 * @return eg X.Y.Zd-prerelease
		 */
		public String getVersionString(boolean prerelease, String appCode) {

			StringBuilder suffix = new StringBuilder(10);
			suffix.append(name);
			if (!"classic".equals(appCode)) {
				suffix.append(appCode);
			}
			if (prerelease) {
				suffix.append("-prerelease");
			}

			// everything except old Java desktop version should be version
			// 6.0.x.x
			// change 5.4.274.0 to 6.0.274.0
			return (this == DESKTOP ? VERSION_STRING : getVersionString6()) + "-" + suffix;
		}

		/**
		 *
		 * @return true if this is a phone version.
		 */
		public boolean isPhone() {
			return "i".equals(name);
		}

		/**
		 * @return true if the current platform is either Android or iOS.
		 */
		public boolean isMobile() {
			return this == ANDROID || this == IOS;
		}
	}

	String SUITE_APPNAME = "GeoGebraCalculatorSuite";

	String GRAPHING_APPCODE = "graphing";
	String GEOMETRY_APPCODE = "geometry";
	String G3D_APPCODE = "3d";
	String SCIENTIFIC_APPCODE = "scientific";
	String CAS_APPCODE = "cas";
	String SUITE_APPCODE = "suite";
	String CLASSIC_APPCODE = "classic";
	String EVALUATOR_APPCODE = "evaluator";
	String NOTES_APPCODE = "notes";
	String PROBABILITY_APPCODE = "probability";
	String BAYERN_GRAPHING_APPCODE = "bayern_gr";
	String WTR_APPCODE = "wtr";

	String SUITE_SHORT_NAME = "CalculatorSuite.short";

	String SUITE_URL_NAME = "calculator";

	/** File format version */
	String XML_FILE_FORMAT = "5.0";

	// URLs

	/** http prefix */
	String HTTP = "http://";
	/** https prefix */
	String HTTPS = "https://";
	/** mailto prefix */
	String MAILTO = "mailto:";

	/**
	 * minimal precision in LocusEquation: by default rounding is at least 4
	 * decimals (this changes dynamically when zooming in)
	 */
	long PROVER_MIN_PRECISION = 10000;

	/** CSS class name for GeoGebraWeb container tag */
	String GGM_CLASS_NAME = "geogebraweb";
	/** mimetype of GGB files */
	String GGW_MIME_TYPE = "application/vnd.geogebra.file";

	String BYCS_LICENCE_URL = "https://www.bycs.de/rechtliche-dokumente/board/impressum/index.html";

	// //////////////////////////////////////////////////////////////////////////
	// AUTHENTICATING WITH GOOGLE
	// ///////////////////////////////////////////////
	// //////////////////////////////////////////////////////////////////////////

	/**
	 * This app's personal client ID assigned by the Google APIs Console
	 * (http://code.google.com/apis/console).
	 */
	String GOOGLE_CLIENT_ID =
			"656990710877-g0tjpnhriv39e59f5s5ubs81sv2686m6" + ".apps.googleusercontent.com";

	/**
	 * The Graphing Chrome app's client ID from the Google APIs Console
	 * (http://code.google.com/apis/console).
	 */
	String CHROME_APP_CLIENT_ID =
			"656990710877-3uu4empvnqi7co987usqk0talj3hnt2r.apps.googleusercontent.com";

	// The auth scope being requested. This scope will allow the application to
	// identify who the authenticated user is.
	String PLUS_ME_SCOPE = "https://www.googleapis.com/auth/plus.me";
	String DRIVE_SCOPE =
			"https://www.googleapis.com/auth/drive.file https://www.googleapis.com/auth/drive.readonly";
	String USERINFO_EMAIL_SCOPE = "https://www.googleapis.com/auth/userinfo.email";
	String USERINFO_PROFILE_SCOPE = "https://www.googleapis.com/auth/userinfo.profile";


	/**
	 * Get the version string for versions 6.0.*
	 *
	 * @return version string
	 */
	static String getVersionString6() {
		return VERSION_STRING.replace("5.4.", "6.0.");
	}
}
