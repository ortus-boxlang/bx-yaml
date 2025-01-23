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

import java.nio.charset.Charset;

import ortus.boxlang.runtime.bifs.BIF;
import ortus.boxlang.runtime.bifs.BoxBIF;
import ortus.boxlang.runtime.context.IBoxContext;
import ortus.boxlang.runtime.scopes.ArgumentsScope;
import ortus.boxlang.runtime.scopes.Key;
import ortus.boxlang.runtime.types.Argument;
import ortus.boxlang.yaml.YamlParser;

@BoxBIF
public class YamlDeserializeFile extends BIF {

	private static YamlParser parser = YamlParser.getInstance();

	/**
	 * Constructor
	 */
	public YamlDeserializeFile() {
		super();
		declaredArguments = new Argument[] {
		    new Argument( true, "string", Key.filepath ),
		    new Argument( false, "string", Key.charset, Charset.defaultCharset().toString() )
		};
	}

	/**
	 * Deserializes a YAML string into a BoxLang variable.
	 *
	 * @param context   The context in which the BIF is being invoked.
	 * @param arguments Argument scope for the BIF.
	 *
	 * @attribute.filepath The path to the file to deserialize.
	 *
	 * @attribute.charset The charset to use when reading the file. Will default to the system default charset if not provided.
	 *
	 * @return A BoxLang variable.
	 */
	public Object _invoke( IBoxContext context, ArgumentsScope arguments ) {
		return parser.deserializeFromFile( context, arguments.getAsString( Key.filepath ), arguments.getAsString( Key.charset ) );
	}

}
