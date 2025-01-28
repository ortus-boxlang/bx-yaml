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

import java.lang.reflect.Modifier;
import java.util.Arrays;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

import org.yaml.snakeyaml.DumperOptions;
import org.yaml.snakeyaml.nodes.Node;
import org.yaml.snakeyaml.representer.Representer;

import ortus.boxlang.runtime.BoxRuntime;
import ortus.boxlang.runtime.context.IBoxContext;
import ortus.boxlang.runtime.dynamic.casters.BooleanCaster;
import ortus.boxlang.runtime.dynamic.casters.StringCaster;
import ortus.boxlang.runtime.interop.DynamicObject;
import ortus.boxlang.runtime.runnables.IClassRunnable;
import ortus.boxlang.runtime.scopes.Key;
import ortus.boxlang.runtime.scopes.VariablesScope;
import ortus.boxlang.runtime.types.Array;
import ortus.boxlang.runtime.types.DateTime;
import ortus.boxlang.runtime.types.Function;
import ortus.boxlang.runtime.types.IStruct;
import ortus.boxlang.runtime.types.Property;
import ortus.boxlang.runtime.types.Query;
import ortus.boxlang.runtime.types.Struct;
import ortus.boxlang.runtime.types.util.BLCollector;
import ortus.boxlang.yaml.util.KeyDictionary;

public class BoxLangRepresenter extends Representer {

	/**
	 * Constructor
	 */
	public BoxLangRepresenter( DumperOptions options ) {
		super( options );
		this.representers.put( Query.class, new RepresentQuery() );
		this.representers.put( Key.class, new RepesentKey() );
		this.representers.put( DateTime.class, new RepresentDateTime() );
		this.representers.put( Function.class, new RepresentFunction() );
		this.representers.put( DynamicObject.class, new RepresentDynamicObject() );
		this.representers.put( IClassRunnable.class, new RepresetClassRunnable() );
	}

	/**
	 * Inflate an annotation value into an Array
	 *
	 * @param value The value to inflate
	 *
	 * @return The inflated array
	 */
	private static Array inflateArray( Object value ) {
		// If the value is already an array, then cast it
		if ( value instanceof Array castedArray ) {
			return castedArray;
		}

		// Split the string by comma and trim the values
		return Arrays.stream( StringCaster.cast( value ).split( "," ) )
		    .map( String::trim )
		    .collect( BLCollector.toArray() );
	}

	/**
	 * This class is used to represent a BoxClass object as a yaml string
	 */
	private class RepresetClassRunnable extends RepresentString {

		// ThreadLocal to keep track of seen structs in the current thread
		private static final ThreadLocal<Set<IClassRunnable>> visitedClasses = ThreadLocal.withInitial( HashSet::new );

		@Override
		public Node representData( Object data ) {
			if ( data instanceof IClassRunnable bxClass ) {
				Map<Key, Property>	properties			= bxClass.getProperties();
				IStruct				classAnnotations	= bxClass.getAnnotations();
				VariablesScope		variablesScope		= bxClass.getVariablesScope();
				IBoxContext			boxContext			= BoxRuntime.getInstance().getRuntimeContext();

				// Get the current thread's set of visted classes
				Set<IClassRunnable>	visited				= visitedClasses.get();

				if ( visited.contains( bxClass ) ) {
					return super.representData( "recursive-class-skipping" );
				}

				// Verify if the class is NOT serializable via the "serializable" annotation and it's false, return {}
				if ( BooleanCaster.cast( classAnnotations.getOrDefault( Key.serializable, true ) ) == false ) {
					return super.representData( new Struct() );
				}

				// Seed the class annotations needed
				Array classYamlExcludes = inflateArray( classAnnotations.getOrDefault( KeyDictionary.yamlExclude, "" ) );
				// If there is a "toYaml" method in the class, then call it
				// The user wants control over the serialization
				if ( variablesScope.containsKey( KeyDictionary.toYaml ) ) {
					return super.representData(
					    variablesScope.dereferenceAndInvoke( boxContext, KeyDictionary.toYaml, new Object[] { boxContext, this }, false )
					);
				}

				// Filter the variables scope with the properties
				IStruct	memento		= variablesScope.entrySet().stream()
				    // Filter only the properties for the class
				    .filter( entry -> properties.containsKey( entry.getKey() ) )
				    // Filter out any properties that have the yamlEclude annotation
				    .filter( entry -> {
					    Property prop = properties.get( entry.getKey() );
					    // Does the property name exist in the yamlExclude list?
					    return !prop.annotations().containsKey( KeyDictionary.yamlExclude ) && classYamlExcludes.findIndex( prop.name(), false ) == 0;
				    } )
				    // Filter out any properties that have the serialiable = false annotation
				    .filter( entry -> {
					    Property prop = properties.get( entry.getKey() );
					    return BooleanCaster.cast( prop.annotations().getOrDefault( Key.serializable, true ) );
				    } )
				    // If the property is null, then set it to an empty string
				    .map( entry -> {
					    if ( entry.getValue() == null ) {
						    entry.setValue( "" );
					    }
					    return entry;
				    } )
				    // Collect to a struct object
				    .collect(
				        BLCollector.toStruct()
				    );

				// logger.debug( "BoxClassSerializer.writeValue: {}", memento.asString() );

				// Iterate and output each name using the entry set
				Node	nodeReturn	= super.representData( memento );

				// Cleanup Recursion
				visited.remove( bxClass );

				return nodeReturn;
			}
			return super.representData( data );
		}
	}

	/**
	 * This class is used to represent a DynamicObject object as a yaml string
	 */
	private class RepresentDynamicObject extends RepresentString {

		@Override
		public Node representData( Object data ) {
			if ( data instanceof DynamicObject castedDynamicObject ) {
				// If the object is a BoxClass, then serialize it as a BoxClass
				if ( castedDynamicObject.unWrap() instanceof IClassRunnable bxClass ) {
					return super.representData( bxClass );
				}

				// If it's a list, then serialize it as a list
				if ( castedDynamicObject.unWrap() instanceof List<?> castedList ) {
					return super.representData( castedList );
				}

				// If it's a map, then serialize it as a map
				if ( castedDynamicObject.unWrap() instanceof Map<?, ?> castedMap ) {
					return super.representData( castedMap );
				}

				IStruct result = new Struct();
				castedDynamicObject.getFieldsAsStream()
				    // Fiter ONLY public fields
				    .filter( field -> Modifier.isPublic( field.getModifiers() ) )
				    // Write it to the YAML
				    .forEach( field -> result.put(
				        field.getName(),
				        castedDynamicObject.getField( field.getName() ).orElse( "" ).toString()
				    ) );
				return super.representData( result );
			}
			return super.representData( data );
		}

	}

	/**
	 * This class is used to represent a Function object as a yaml string
	 */
	private class RepresentFunction extends RepresentString {

		@Override
		public Node representData( Object data ) {
			if ( data instanceof Function castedFunction ) {
				IStruct result = Struct.of(
				    "function", castedFunction.getBoxMeta().getMeta()
				);
				return super.representData( result );
			}
			return super.representData( data );
		}

	}

	/**
	 * This class is used to represent a Query object as a yaml string
	 */
	private class RepresentQuery extends RepresentString {

		@Override
		public Node representData( Object data ) {
			if ( data instanceof Query castedQuery ) {
				return super.representData( castedQuery.asArrayOfStructs() );
			}
			return super.representData( data );
		}

	}

	/**
	 * This class is used to represent a BoxLang DateTime object as a string.
	 */
	private class RepresentDateTime extends RepresentString {

		@Override
		public Node representData( Object data ) {
			if ( data instanceof DateTime castedDateTime ) {
				return super.representData( castedDateTime.toISOString() );
			}
			return super.representData( data );
		}
	}

	/**
	 * This class is used to represent a Key object as a string.
	 */
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
