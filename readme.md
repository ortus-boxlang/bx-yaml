# ⚡︎ BoxLang Module: BoxLang YAML Support

```
|:------------------------------------------------------:|
| ⚡︎ B o x L a n g ⚡︎
| Dynamic : Modular : Productive
|:------------------------------------------------------:|
```

<blockquote>
	Copyright Since 2023 by Ortus Solutions, Corp
	<br>
	<a href="https://www.boxlang.io">www.boxlang.io</a> |
	<a href="https://www.ortussolutions.com">www.ortussolutions.com</a>
</blockquote>

<p>&nbsp;</p>

## Welcome to the BoxLang YAML Module

This module provides a YAML parser and emitter for BoxLang.  It is based on the [SnakeYAML](https://bitbucket.org/asomov/snakeyaml) library and provides a simple way to parse and emit YAML content in BoxLang.

## Installation

You can install it via the BoxLang CLI:

```bash
install-bx-module bx-yaml
```

Or you can install it into a CommandBox server:

```bash
box install boxlang-yaml
```

## Usage

This module registers the following BIFS:

- `yamlSerialize( content, [filepath], [charset=utf8] ):yaml` : Serialize a BoxLang variable into a YAML string.  You can also serialize to a file if you provide a file path.
- `yamlDeserialize( content ):any` : Deserialize a YAML string into a BoxLang variable.
- `yamlDeserializeFile( filepath, [charset=utf8] ):any` : Deserialize a YAML file into a BoxLang variable.

Here is a simple example:

```java
// Serialize a BoxLang structure into a YAML string
yaml = yamlSerialize( { name="Luis", age=21, city="Orlando" } );
// Serialize a more complex structure
yaml = yamlSerialize( { name="Luis", age=21, city="Orlando", address={ street="1234", city="Orlando", state="FL" } } );

// Deserialize a YAML string into a BoxLang structure
data = yamlDeserialize( yaml );

// Deserialize a YAML file into a BoxLang structure
data = yamlDeserializeFile( "data.yml" );
```

## BoxLang Class Serialization

BoxLang classes will be serialized as a structure according to its properties.  However it must adhere to the following rules:

- The class must have a `serializable = true` annotation. or none at all, that is the default.
- The property must have a `serializable = true` annotation or none at all, that is the default.
- The property must NOT exist in the `yamlExclude` list in the class or the parent class.
- The property must NOT have a `yamlExclude` annotation.

## BoxLang Class Custom Serialization

If you are serializing BoxLang classes, you can implement the `toYAML()` method in your classes to provide a custom serialization.  Here is an example:

```java
class {

	function init( required name="", required age=0, required city="" ){
		variables.name = arguments.name;
		variables.age = arguments.age;
		variables.city = arguments.city;
		return this;
	}

	function toYAML(){
		return { name=variables.name, city=variables.city } );
	}

}
```

## Ortus Sponsors

BoxLang is a professional open-source project and it is completely funded by the [community](https://patreon.com/ortussolutions) and [Ortus Solutions, Corp](https://www.ortussolutions.com).  Ortus Patreons get many benefits like a cfcasts account, a FORGEBOX Pro account and so much more.  If you are interested in becoming a sponsor, please visit our patronage page: [https://patreon.com/ortussolutions](https://patreon.com/ortussolutions)

### THE DAILY BREAD

 > "I am the way, and the truth, and the life; no one comes to the Father, but by me (JESUS)" Jn 14:1-12
