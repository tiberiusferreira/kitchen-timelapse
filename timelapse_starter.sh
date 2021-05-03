#! /bin/bash
### BEGIN INIT INFO
# Provides:          timelapse_manufacturing
# Required-Start:    $remote_fs $syslog
# Required-Stop:     $remote_fs $syslog
# Default-Start:     2 3 4 5
# Default-Stop:      0 1 6
# Short-Description: Timelapse manufacturing
# Description:       Timelapse manufacturing
### END INIT INFO

# If you want a command to always run, put it here

# Carry out specific functions when asked to by the system
case "$1" in
  start)
    echo "Starting Timelapse Recording"
    # run application you want to start
    sleep 60 &&\
     cd /home/pi/kitchen-timelapse && (sudo -u pi env RUST_BACKTRACE=1 /home/pi/.cargo/bin/cargo run --bin camera_api --release &
	 sudo -u pi env RUST_BACKTRACE=1 /home/pi/.cargo/bin/cargo run --bin video_streaming_api --release  &)
    ;;
  stop)
    echo "Starting Timelapse Recording"
    # kill application you want to stop
    killall main
    ;;
  *)
    echo "Usage: {start|stop}"
    exit 1
    ;;
esac

exit 0