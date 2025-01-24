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

import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.charset.Charset;

import org.yaml.snakeyaml.DumperOptions;
import org.yaml.snakeyaml.LoaderOptions;
import org.yaml.snakeyaml.Yaml;
import org.yaml.snakeyaml.representer.Representer;

import ortus.boxlang.runtime.context.IBoxContext;
import ortus.boxlang.runtime.types.exceptions.BoxIOException;
import ortus.boxlang.runtime.util.FileSystemUtil;

/**
 * In charge of parsing YAML files.
 */
public class YamlParser {

	/**
	 * This service instance.
	 */
	private static YamlParser	instance	= null;

	/**
	 * The YAML parser.
	 */
	private Yaml				parser;

	/**
	 * Constructor
	 */
	private YamlParser() {
		DumperOptions options = new DumperOptions();
		options.setDefaultFlowStyle( DumperOptions.FlowStyle.BLOCK );
		options.setPrettyFlow( true );

		Representer representer = new BoxLangRepresenter( options );
		representer.getPropertyUtils().setSkipMissingProperties( true );

		LoaderOptions loaderOptions = new LoaderOptions();
		loaderOptions.setAllowDuplicateKeys( false );
		loaderOptions.setAllowRecursiveKeys( false );
		loaderOptions.setEnumCaseSensitive( false );

		this.parser = new Yaml(
		    new BoxLangConstructor( Object.class, loaderOptions ),
		    representer,
		    options
		);
	}

	/**
	 * Get the singleton instance of the parser.
	 *
	 * @return The parser instance.
	 */
	public static YamlParser getInstance() {
		if ( instance == null ) {
			instance = new YamlParser();
		}
		return instance;
	}

	/**
	 * Serialize an object to a YAML string.
	 *
	 * @param context The context of execution
	 * @param obj     The object to serialize.
	 *
	 * @return The YAML string.
	 */
	public String serialize( IBoxContext context, Object obj ) {
		return this.parser.dump( obj );
	}

	/**
	 * Serialize an object directly to a file destination path.
	 *
	 * @param obj     The object to serialize.
	 * @param path    The absolute path to serialize the object to.
	 * @param charset The charset to use when writing the file.
	 */
	public void serializeToFile( IBoxContext context, Object obj, String path, String charset ) {
		String filePath = FileSystemUtil.expandPath( context, path ).absolutePath().toString();

		try ( FileWriter writer = new FileWriter( filePath, Charset.forName( charset ) ) ) {
			this.parser.dump( obj, writer );
		} catch ( IOException e ) {
			throw new BoxIOException( "Error serializing yaml", e );
		}
	}

	/**
	 * Deserialize a YAML string to an object.
	 *
	 * @param context The context of execution
	 * @param yaml    The YAML string to deserialize.
	 *
	 * @return The deserialized BoxLang Object.
	 */
	public Object deserialize( IBoxContext context, String yaml ) {
		return this.parser.load( yaml );
	}

	/**
	 * Deserialize a YAML file to an object.
	 *
	 * @param context The context of execution
	 * @param path    The absolute path to the YAML file.
	 * @param charset The charset to use when reading the file.
	 *
	 * @return The deserialized BoxLang Object.
	 */
	public Object deserializeFromFile( IBoxContext context, String path, String charset ) {
		String filePath = FileSystemUtil.expandPath( context, path ).absolutePath().toString();

		try ( FileReader reader = new FileReader( filePath, Charset.forName( charset ) ) ) {
			return this.parser.load( reader );
		} catch ( IOException e ) {
			throw new BoxIOException( "Error deserializing yaml", e );
		}
	}

}
