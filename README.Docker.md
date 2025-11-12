# Docker Usage Guide for RustCityGML2OBJ# Docker Usage Guide for RustCityGML2OBJ



## Building the Docker Image## Building the Docker Image



```bash```bash

docker build -t rustcitygml2obj:latest .docker build -t rustcitygml2obj:latest .

``````



## Running with Docker## Running with Docker



### Basic Usage### Option 1: Using Docker Run



```bashBasic usage:

docker run --rm \```bash

  -v /absolute/path/to/input:/input:ro \docker run --rm \

  -v /absolute/path/to/output:/output \  -v /absolute/path/to/input:/input:ro \

  rustcitygml2obj:latest \  -v /absolute/path/to/output:/output \

  --input /input/your-file.gml \  rustcitygml2obj:latest \

  --output /output  --input /input/your-file.gml \

```  --output /output

```

### With Optional Features

With optional features:

```bash```bash

docker run --rm \docker run --rm \

  -v /absolute/path/to/input:/input:ro \  -v /absolute/path/to/input:/input:ro \

  -v /absolute/path/to/output:/output \  -v /absolute/path/to/output:/output \

  rustcitygml2obj:latest \  rustcitygml2obj:latest \

  --input /input/your-file.gml \  --input /input/your-file.gml \

  --output /output \  --output /output \

  --tbw --add_bb --add_json  --tbw --add_bb --add_json

``````



### With Grouping Options### Option 2: Using Docker Compose



```bash1. Create `input` and `output` directories in the project root:

# Group by semantic class```bash

docker run --rm \mkdir -p input output

  -v $(pwd)/input:/input:ro \```

  -v $(pwd)/output:/output \

  rustcitygml2obj:latest \2. Place your CityGML files in the `input` directory

  --input /input/building.gml \

  --output /output \3. Edit `docker-compose.yml` and uncomment/modify the command section:

  --group-sc```yaml

command: ["--input", "/input/your-file.gml", "--output", "/output"]

# Group by semantic component```

docker run --rm \

  -v $(pwd)/input:/input:ro \4. Run with docker-compose:

  -v $(pwd)/output:/output \```bash

  rustcitygml2obj:latest \docker-compose up

  --input /input/building.gml \```

  --output /output \

  --group-scompOr build and run:

``````bash

docker-compose up --build

## Volume Mounts```



The Docker container expects two volume mounts:## Volume Mounts

- `/input` - Your CityGML input files (mounted read-only)

- `/output` - Where the OBJ files will be writtenThe Docker container expects two volume mounts:

- `/input` - Your CityGML input files (mounted read-only)

## Examples- `/output` - Where the OBJ files will be written



### Example 1: Simple Conversion## Examples

```bash

docker run --rm \### Example 1: Simple Conversion

  -v $(pwd)/input:/input:ro \```bash

  -v $(pwd)/output:/output \docker run --rm \

  rustcitygml2obj:latest \  -v $(pwd)/input:/input:ro \

  --input /input/building.gml \  -v $(pwd)/output:/output \

  --output /output  rustcitygml2obj:latest \

```  --input /input/building.gml \

  --output /output

### Example 2: With Building-wise Translation```

```bash

docker run --rm \### Example 2: With Building-wise Translation

  -v $(pwd)/input:/input:ro \```bash

  -v $(pwd)/output:/output \docker run --rm \

  rustcitygml2obj:latest \  -v $(pwd)/input:/input:ro \

  --input /input/building.gml \  -v $(pwd)/output:/output \

  --output /output \  rustcitygml2obj:latest \

  --tbw  --input /input/building.gml \

```  --output /output \

  --tbw

### Example 3: With All Optional Features```

```bash

docker run --rm \### Example 3: With All Optional Features

  -v $(pwd)/input:/input:ro \```bash

  -v $(pwd)/output:/output \docker run --rm \

  rustcitygml2obj:latest \  -v $(pwd)/input:/input:ro \

  --input /input/building.gml \  -v $(pwd)/output:/output \

  --output /output \  rustcitygml2obj:latest \

  --tbw --add_bb --add_json --group-sc  --input /input/building.gml \

```  --output /output \

  --tbw --add_bb --add_json

## Troubleshooting```



### Permission Issues## Troubleshooting

If you encounter permission issues with the output directory:

```bash### Permission Issues

# On Linux, you might need to adjust permissionsIf you encounter permission issues with the output directory:

chmod 777 output```bash

```# On Linux, you might need to adjust permissions

chmod 777 output

### Check if the image was built correctly```

```bash

docker images | grep rustcitygml2obj### Check if the image was built correctly

``````bash

docker images | grep rustcitygml2obj

### View help```

```bash

docker run --rm rustcitygml2obj:latest --help### View help

``````bash

docker run --rm rustcitygml2obj:latest --help

## Image Size Optimization```



The Dockerfile uses a multi-stage build to keep the final image size small:## Image Size Optimization

- Build stage: Uses full Rust toolchain with nightly compiler

- Runtime stage: Only includes the compiled binary and minimal dependenciesThe Dockerfile uses a multi-stage build to keep the final image size small:

- Build stage: Uses full Rust toolchain

To check the image size:- Runtime stage: Only includes the compiled binary and minimal dependencies

```bash

docker images rustcitygml2obj:latestTo check the image size:

``````bash

docker images rustcitygml2obj:latest
```
