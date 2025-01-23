/**
 * [BoxLang]
 *
 * Copyright [2023] [Ortus Solutions, Corp]
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
package ortus.boxlang.yaml.bifs;

import ortus.boxlang.runtime.bifs.BIF;
import ortus.boxlang.runtime.bifs.BoxBIF;
import ortus.boxlang.runtime.context.IBoxContext;
import ortus.boxlang.runtime.scopes.ArgumentsScope;
import ortus.boxlang.runtime.scopes.Key;
import ortus.boxlang.runtime.types.Argument;
import ortus.boxlang.yaml.YamlParser;

@BoxBIF
public class YamlDeserialize extends BIF {

	private static YamlParser parser = YamlParser.getInstance();

	/**
	 * Constructor
	 */
	public YamlDeserialize() {
		super();
		declaredArguments = new Argument[] {
		    new Argument( true, "string", Key.content )
		};
	}

	/**
	 * Deserializes a YAML string into a BoxLang variable.
	 *
	 * @param context   The context in which the BIF is being invoked.
	 * @param arguments Argument scope for the BIF.
	 *
	 * @attribute.yaml The YAML string to deserialize.
	 *
	 * @return A BoxLang variable.
	 */
	public Object _invoke( IBoxContext context, ArgumentsScope arguments ) {
		return parser.deserialize( context, arguments.getAsString( Key.content ) );
	}

}
