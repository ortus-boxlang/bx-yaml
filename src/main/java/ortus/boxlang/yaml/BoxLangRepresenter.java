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

import org.yaml.snakeyaml.DumperOptions;
import org.yaml.snakeyaml.nodes.Node;
import org.yaml.snakeyaml.representer.Representer;

import ortus.boxlang.runtime.scopes.Key;

public class BoxLangRepresenter extends Representer {

	/**
	 * Constructor
	 */
	public BoxLangRepresenter( DumperOptions options ) {
		super( options );
		this.representers.put( Key.class, new RepesentKey() );
	}

	private class RepesentKey extends RepresentString {

		@Override
		public Node representData( Object data ) {
			if ( data instanceof Key castedKey ) {
				return super.representData( castedKey.getName() );
			}
			return super.representData( data );
		}
	}

}
