/**
 * [BoxLang]
 *
 * Copyright [2023] [Ortus Solutions, Corp]
 *
 * Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the
 * License. You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS"
 * BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language
 * governing permissions and limitations under the License.
 */
package ortus.boxlang.moduleslug;

import static com.google.common.truth.Truth.assertThat;

import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

import ortus.boxlang.runtime.scopes.Key;
import ortus.boxlang.runtime.types.IStruct;

/**
 * This loads the module and runs an integration test on the module.
 */
public class IntegrationTest extends BaseIntegrationTest {

	@DisplayName( "Test the module loads in BoxLang" )
	@Test
	public void testModuleLoads() {
		// Then
		assertThat( moduleService.getRegistry().containsKey( moduleName ) ).isTrue();
	}

	@DisplayName( "Test that you can call yamlSerialize" )
	@Test
	public void testYamlSerialize() {
		runtime.executeSource(
		    """
		    	result = yamlSerialize( null )
		    """,
		    context );
		assertThat( variables.getAsString( result ) ).contains( "null" );
	}

	@DisplayName( "It can serialize a string" )
	@Test
	public void testCanSerializeString() {
		runtime.executeSource(
		    """
		    	result = yamlSerialize( "Hello World" )
		    """,
		    context );
		assertThat( variables.get( result ) ).isEqualTo( "Hello World\n" );
	}

	@DisplayName( "It can serialize a number" )
	@Test
	public void testCanSerializeNumber() {
		// @formatter:off
		runtime.executeSource(
		    """
		        result = yamlSerialize( 42 )
		    	println( result )
		    """,
		    context );
		// @formatter:on
		assertThat( variables.get( result ) ).isEqualTo( "42\n" );
	}

	@DisplayName( "It can serialize an array" )
	@Test
	public void testCanSerializeArray() {
		// @formatter:off
		runtime.executeSource(
		    """
		        result = yamlSerialize( [ 1, 2, 3 ] )
		    	println( result )
		    """,
		    context );
		// @formatter:on
		assertThat( variables.get( result ) ).isEqualTo( "- 1\n- 2\n- 3\n" );
	}

	@DisplayName( "It can serialize a struct" )
	@Test
	public void testCanSerializeStruct() {
		// @formatter:off
		runtime.executeSource(
		    """
		        result = yamlSerialize( { name = "Luis", age = 42 } )
		    	println( result )
		    """,
		    context );
		// @formatter:on
		assertThat( variables.get( result ) ).isEqualTo( "age: 42\nname: Luis\n" );
	}

	@DisplayName( "It can serialize dates" )
	@Test
	public void testCanSerializeDates() {
		// @formatter:off
		runtime.executeSource(
		    """
		        setTimezone( "UTC" );
		       	result = YamlSerialize( createDate( 2024, 1, 1 ) )
		    	println( result )
		    """,
		    context );
		// @formatter:on
		assertThat( variables.getAsString( result ) ).contains( "'2024-01-01T00:00:00Z'" );
	}

	@DisplayName( "It can serialize a struct with data types" )
	@Test
	public void testCanSerializeStructWithTypes() {
		// @formatter:off
		runtime.executeSource(
		    """
				setTimezone( "UTC" );
		        result = yamlSerialize( {
					name = "Luis",
					numbers = [ 1, 2, 3 ],
					age = 42,
					isOpen = true,
					nested = {
						foo = "bar"
					},
					dob = createDate( 2024, 1, 1 )
				} )
		    	println( result )

				bx = yamlDeserialize( result )
				println( bx )
		    """,
		    context );
		// @formatter:on
		IStruct bxResult = ( IStruct ) variables.get( bx );
		assertThat( bxResult.get( new Key( "name" ) ) ).isEqualTo( "Luis" );
		assertThat( bxResult.get( new Key( "numbers" ) ).toString() ).isEqualTo( "[[1, 2, 3]]" );
		assertThat( bxResult.get( new Key( "age" ) ) ).isEqualTo( 42 );
		assertThat( bxResult.get( new Key( "isOpen" ) ) ).isEqualTo( true );
		assertThat( bxResult.get( new Key( "nested" ) ).toString().replaceAll( "\\s", "" ) ).isEqualTo( "{foo:\"bar\"}" );
		assertThat( bxResult.get( new Key( "dob" ) ) ).isEqualTo( "2024-01-01T00:00:00Z" );
	}

	@DisplayName( "It can serialize a query as array of structs" )
	@Test
	public void testCanSerializeQuery() {
		// @formatter:off
		runtime.executeSource(
		    """
		        result = yamlSerialize(
					queryNew(
						"col1,col2,col3",
						"numeric,varchar,bit",
						[
							[1,"brad",true],
							[2,"wood",false]
						]
					)
				)
		    	println( result )
				bx = yamlDeserialize( result )
				println( bx )
		    """,
		    context );
		// @formatter:on
	}

	@DisplayName( "Deserialize a test file" )
	@Test
	public void testDeserializeFile() {
		// @formatter:off
		runtime.executeSource(
		    """
		        result = yamlDeserializeFile( "src/test/resources/test.yml" )
		    	println( result )
		    """,
		    context );
		// @formatter:on
	}

}
