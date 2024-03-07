# Protobuf files

## How to add a new protobuf file

1.  Create a few called <package_name>.proto in protos/
2.  Make sure to use the same "package" pragma as the file name in the body of
    the proto
3.  Specify any message schemas in that proto file.

How to use the resultant protobufs:

This package will automatically generate modules that contain all your message
structs. So given a file called foo.proto with `message Bar`, your crate can
import the generated struuct like so:

    use pb::foo::Bar;
