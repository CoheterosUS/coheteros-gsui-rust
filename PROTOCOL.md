# Protocol Specification

Little-Endian

float: 4 bytes, IEEE 754

int8: 1 byte, Two's Complement
int16: 2 bytes, Two's Complement
int32: 4 bytes, Two's Complement

uint8: 1 byte
uint16: 2 bytes
uint32: 4 bytes

Any enum, bitmask, or structure is subject to change without notice.

## SystemState (Enum)

| Value | Name             |
|-------|------------------|
| 0     | IDLE             |
| 1     | CALIBRATION      |
| 2     | PRELAUNCH        |
| 3     | BURN             |
| 4     | PASSIVE_BURNOUT  |
| 5     | ACTIVE_BURNOUT   |
| 6     | APOGEE           |
| 7     | PARACHUTE        |
| 8     | LANDED           |
| 9     | GROUND_ABORT     |
| 10    | DESCENT_ABORT    |

## CommandType (Enum)

| Value  | Name                 | Notes                     |
|--------|----------------------|---------------------------|
| `0x00` | COMMAND_NONE         |                           |
| `0x01` | COMMAND_RESET        |                           |
| `0x02` | COMMAND_GROUND_ABORT |                           |
| `0x03` | COMMAND_CALIBRATION  |                           |
| `0x04` | COMMAND_DROGUE       |                           |
| `0x05` | COMMAND_LANDED       |                           |
| `0x10` | COMMAND_HIL_DATA     | Excluded From LastCommand |
| `0x20` | COMMAND_GPS_DATA     | Excluded From LastCommand |

## SystemFaultFlags (Bitmask)

| Bit | Flag                              |
|-----|-----------------------------------|
| 0   | BMP280_MODE_IDLE_FAILED           |
| 1   | BMP280_MODE_PERFORMANCE_FAILED    |
| 2   | BMP581_MODE_IDLE_FAILED           |
| 3   | BMP581_MODE_PERFORMANCE_FAILED    |
| 4   | IIM42653_MODE_IDLE_FAILED         |
| 5   | IIM42653_MODE_PERFORMANCE_FAILED  |
| 6   | IIS2MDCTR_MODE_IDLE_FAILED        |
| 7   | IIS2MDCTR_MODE_PERFORMANCE_FAILED |
| 8   | SD_MOUNT_FAILED                   |
| 9   | SD_OPEN_FAILED                    |

## RelayState (Bitmask)

| Bit | Flag             |
|-----|------------------|
| 0   | DROGUE_FIRED     |
| 1   | PARACHUTE_FIRED  |

## Flight Record (Internal, Packed)

| Field          | Type   | Unit                | Notes            |
|----------------|--------|---------------------|------------------|
| Sync           | uint16 |                     | `0xCAFE`         |
| Tick           | uint32 | ms                  | FreeRTOS Tick    |
| AccelX         | float  | m/s²                |                  |
| AccelY         | float  | m/s²                |                  |
| AccelZ         | float  | m/s²                |                  |
| GyroX          | float  | Degrees/s           |                  |
| GyroY          | float  | Degrees/s           |                  |
| GyroZ          | float  | Degrees/s           |                  |
| MagX           | float  | Milligauss          |                  |
| MagY           | float  | Milligauss          |                  |
| MagZ           | float  | Milligauss          |                  |
| PressurePa     | float  | Pascals             |                  |
| TemperatureC   | float  | Celsius             |                  |
| Latitude       | int32  | Degrees × 10^7      |                  |
| Longitude      | int32  | Degrees × 10^7      |                  |
| GPSAltitude    | float  | Meters              |                  |
| UnixTime       | uint32 | Epoch Seconds       |                  |
| Milliseconds   | uint16 | 0–999               |                  |
| Satellites     | uint8  | Count               |                  |
| BaroAltitude   | float  | Meters              |                  |
| BaroVelocity   | float  | m/s                 |                  |
| VelX           | float  | m/s                 |                  |
| VelY           | float  | m/s                 |                  |
| VelZ           | float  | m/s                 |                  |
| FaultFlags     | uint32 | Bitmask             | SystemFaultFlags |
| BatteryVoltage | float  | Volts               |                  |
| State          | uint8  | SystemState         |                  |
| RelayState     | uint8  | Bitmask             | RelayState       |
| LastCommand    | uint8  | CommandType         | Persists         |
| SyncEnd        | uint8  |                     | `0xBE`           |

## Command Frame (Structure, Packed)

| Offset | Size | Type  | Value  | Description         |
|--------|------|-------|--------|---------------------|
| 0      | 1    | uint8 | `0xFE` | Sync LSB            |
| 1      | 1    | uint8 | `0xCA` | Sync MSB            |
| 2      | 1    | uint8 | CMD    | CommandType         |
| 3      | 1    | uint8 | `0x00` | Payload Length (0)  |
| 4      | 1    | uint8 | `0xBE` | Footer              |

## Wire Telemetry Packet (Structure, Packed)

| Offset | Size | Type   | Field            | Encoding          |
|--------|------|--------|------------------|-------------------|
| 0      | 2    | uint16 | Sync             | `0xCAFE`          |
| 2      | 4    | uint32 | Tick             | Raw               |
| 6      | 4    | int32  | AccelX           | ×100              |
| 10     | 4    | int32  | AccelY           | ×100              |
| 14     | 4    | int32  | AccelZ           | ×100              |
| 18     | 4    | int32  | GyroX            | ×100              |
| 22     | 4    | int32  | GyroY            | ×100              |
| 26     | 4    | int32  | GyroZ            | ×100              |
| 30     | 4    | int32  | MagX             | ×100              |
| 34     | 4    | int32  | MagY             | ×100              |
| 38     | 4    | int32  | MagZ             | ×100              |
| 42     | 4    | int32  | PressurePa       | ×100              |
| 46     | 4    | int32  | TemperatureC     | ×100              |
| 50     | 4    | int32  | Latitude         | ×10^7             |
| 54     | 4    | int32  | Longitude        | ×10^7             |
| 58     | 4    | int32  | GPSAltitude      | ×100              |
| 62     | 1    | uint8  | Satellites       | Raw               |
| 63     | 4    | int32  | BaroAltitude     | ×100              |
| 67     | 4    | int32  | BaroVelocity     | ×100              |
| 71     | 4    | int32  | VelX             | ×100              |
| 75     | 4    | int32  | VelY             | ×100              |
| 79     | 4    | int32  | VelZ             | ×100              |
| 83     | 4    | uint32 | Flags            | Bitmask           |
| 87     | 4    | int32  | BatteryVoltage   | ×100              |
| 91     | 1    | uint8  | State            | Enum              |
| 92     | 1    | uint8  | RelayState       | Bitmask           |
| 93     | 1    | uint8  | LastCommand      | Enum              |
| 94     | 1    | uint8  | SyncEnd          | `0xBE`            |

## Wire SD Log Record (Structure, Packed)

| Offset | Size | Type    | Field          | Encoding             |
|--------|------|---------|----------------|----------------------|
| 0      | 2    | uint16  | Sync           | `0xCAFE`             |
| 2      | 4    | uint32  | Tick           | Raw                  |
| 6      | 4    | float32 | AccelX         | Native               |
| 10     | 4    | float32 | AccelY         | Native               |
| 14     | 4    | float32 | AccelZ         | Native               |
| 18     | 4    | float32 | GyroX          | Native               |
| 22     | 4    | float32 | GyroY          | Native               |
| 26     | 4    | float32 | GyroZ          | Native               |
| 30     | 4    | float32 | MagX           | Native               |
| 34     | 4    | float32 | MagY           | Native               |
| 38     | 4    | float32 | MagZ           | Native               |
| 42     | 4    | float32 | PressurePa     | Native               |
| 46     | 4    | float32 | TemperatureC   | Native               |
| 50     | 4    | int32   | Latitude       | ×10^7                |
| 54     | 4    | int32   | Longitude      | ×10^7                |
| 58     | 4    | float32 | GPSAltitude    | Native               |
| 62     | 4    | uint32  | UnixTime       | Epoch Seconds        |
| 66     | 2    | uint16  | Milliseconds   | 0–999                |
| 68     | 1    | uint8   | Satellites     | Raw                  |
| 69     | 4    | uint32  | Flags          | Bitmask              |
| 73     | 4    | float32 | BatteryVoltage | Native               |
| 77     | 1    | uint8   | State          | Enum                 |
| 78     | 1    | uint8   | RelayState     | Bitmask              |
| 79     | 1    | uint8   | LastCommand    | Enum                 |
| 80     | 1    | uint8   | SyncEnd        | `0xBE`               |
