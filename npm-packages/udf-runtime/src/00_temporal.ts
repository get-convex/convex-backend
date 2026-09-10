import { performOp } from "udf-syscall-ffi";

export function setupTemporal(global: any) {
  const Temporal = global.Temporal;

  const fromEpochMilliseconds = Temporal.Instant.fromEpochMilliseconds;
  // Native Now methods read V8's platform clock independently of Date.now.
  // Override them to return the frozen UDF time and record that time was observed.
  const instant = () => fromEpochMilliseconds(performOp("now"));
  const zonedDateTimeISO = (timeZone: any = "UTC") =>
    instant().toZonedDateTimeISO(timeZone);

  Temporal.Now = Object.defineProperties(
    {},
    {
      instant: {
        value: instant,
        writable: true,
        configurable: true,
      },
      timeZoneId: {
        value: Temporal.Now.timeZoneId,
        writable: true,
        configurable: true,
      },
      plainDateTimeISO: {
        value: (timeZone: any = "UTC") =>
          zonedDateTimeISO(timeZone).toPlainDateTime(),
        writable: true,
        configurable: true,
      },
      zonedDateTimeISO: {
        value: zonedDateTimeISO,
        writable: true,
        configurable: true,
      },
      plainDateISO: {
        value: (timeZone: any = "UTC") =>
          zonedDateTimeISO(timeZone).toPlainDate(),
        writable: true,
        configurable: true,
      },
      plainTimeISO: {
        value: (timeZone: any = "UTC") =>
          zonedDateTimeISO(timeZone).toPlainTime(),
        writable: true,
        configurable: true,
      },
      [Symbol.toStringTag]: {
        value: "Temporal.Now",
        configurable: true,
      },
    },
  );
}
