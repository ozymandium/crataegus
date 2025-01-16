GpsLogger Settings
===

## Settings
### General Options
- Start on bootup: true
- Start on app launch: true
- Coordinates display format: -12.345678
### Logging Details
- Log to custom URL: true
- Log to CSV: true
- New file creation: Once a day
- Log time with timezone offset: true
- 
#### Custom URL
- Log to custom URL: true
- Allow auto sending: true
- Discard offline locations: false
##### HTTP Body
```json
{
    "time": %TIMESTAMP,
    "latitude": %LAT,
    "longitude": %LON,
    "altitude": %ALT,
    "accuracy": %ACC
}
```
##### HTTP Headers
```
Content-Type: application/json
```
### Performance
- Log GPS/GNSS Locations: true
- Log network locations: true
- Log passive locations: true
- Logging interval: 60s
- Keep GPS on between fixes: false
- Distance filter: 0 meters
- Accuracy filter: 100 meters
- Duration to match accuracy: 60s
- Choose best accuracy in duration: true
- Absolute time to GPS fix: 120s 
- Use MSL instead of WGS84: false
- Subtract altitude offset: 0 meters
### Auto send, email, and upload
- Allow auto sending: true
- How often?: 60 min
- When I press stop: true
- Send zip file: false
- Send on wifi only: false
