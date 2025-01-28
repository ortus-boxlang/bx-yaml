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
package ortus.boxlang.yaml;

import java.util.List;
import java.util.Map;

import org.yaml.snakeyaml.LoaderOptions;
import org.yaml.snakeyaml.constructor.Constructor;
import org.yaml.snakeyaml.nodes.Node;
import org.yaml.snakeyaml.nodes.ScalarNode;
import org.yaml.snakeyaml.nodes.Tag;

import ortus.boxlang.runtime.dynamic.casters.DateTimeCaster;
import ortus.boxlang.runtime.types.Array;
import ortus.boxlang.runtime.types.Struct;

public class BoxLangConstructor extends Constructor {

	public BoxLangConstructor( Class<?> theRoot, LoaderOptions options ) {
		super( theRoot, options );
	}

	@SuppressWarnings( "unchecked" )
	@Override
	protected Object constructObject( Node node ) {
		// Is this a map, map to a Struct
		if ( node.getTag().equals( Tag.MAP ) ) {
			// Use the custom Struct class
			Map<Object, Object> map = ( Map<Object, Object> ) super.constructObject( node );
			return Struct.fromMap( map );
		}
		// Is this a sequence? Map to an Array
		else if ( node.getTag().equals( Tag.SEQ ) ) {
			// Use the custom Array class
			List<Object> list = ( List<Object> ) super.constructObject( node );
			return Array.of( list );
		}
		// If this is a date/time use a BoxLang date/time object
		else if ( node.getTag().equals( Tag.TIMESTAMP ) ) {
			// Handle date/time objects
			String dateString = ( String ) super.constructScalar( ( ScalarNode ) node );
			return DateTimeCaster.cast( dateString );
		}

		return super.constructObject( node );
	}
}
