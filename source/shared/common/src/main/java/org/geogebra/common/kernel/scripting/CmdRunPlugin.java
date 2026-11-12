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

package org.geogebra.common.kernel.scripting;

import java.util.ArrayList;
import java.util.List;

import org.geogebra.common.kernel.Kernel;
import org.geogebra.common.kernel.arithmetic.Command;
import org.geogebra.common.kernel.commands.CmdScripting;
import org.geogebra.common.kernel.geos.GeoElement;
import org.geogebra.common.kernel.geos.GeoText;

/**
 * RunPlugin( &lt;Plugin&gt;, &lt;Command&gt;, &lt;Object&gt;, ... ) runs a command of a Locus plugin.
 */
public class CmdRunPlugin extends CmdScripting {

	/**
	 * @param kernel kernel
	 */
	public CmdRunPlugin(Kernel kernel) {
		super(kernel);
	}

	@Override
	protected GeoElement[] perform(Command c) {
		GeoElement[] args = resArgs(c);
		if (args.length < 2) {
			throw argNumErr(c);
		}
		for (int i = 0; i < 2; i++) {
			if (!(args[i] instanceof GeoText)) {
				throw argErr(c, args[i]);
			}
		}
		List<String> objects = new ArrayList<>();
		for (int i = 2; i < args.length; i++) {
			objects.add(args[i].getLabelSimple());
		}
		app.runPlugin(((GeoText) args[0]).getTextString(), ((GeoText) args[1]).getTextString(),
				objects);
		return args;
	}
}
