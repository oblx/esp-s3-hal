#!/usr/bin/env python3
"""monitor.py — serial monitor for the S3 over /dev/ttyACM0."""
import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = int(sys.argv[2]) if len(sys.argv) > 2 else 115200

def main():
	try:
		with serial.Serial(PORT, BAUD, timeout=1) as ser:
			print(f"monitor {PORT} @ {BAUD} — Ctrl-C to exit")
			while True:
				line = ser.readline()
				if line:
					sys.stdout.write(line.decode("utf-8", errors="replace"))
					sys.stdout.flush()
	except KeyboardInterrupt:
		print("\n-- monitor stopped --")
	except serial.SerialException as e:
		print(f"serial error: {e}", file=sys.stderr)
		sys.exit(1)

if __name__ == "__main__":
	main()
