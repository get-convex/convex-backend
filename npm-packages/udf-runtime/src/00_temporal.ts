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

// `Intl.DateTimeFormat`'s `format` and `formatToParts` format the current time
// when called without a date. V8 reads that from its platform clock rather than
// `Date.now`, which would bypass the frozen UDF time.
export function patchDateTimeFormat(global: any, now: () => number) {
  const proto = global.Intl.DateTimeFormat.prototype;
  const formatGetter = Object.getOwnPropertyDescriptor(proto, "format")!.get!;
  const boundFormat = Symbol("boundFormat");
  Object.defineProperty(proto, "format", {
    get(this: any) {
      const nativeFormat = formatGetter.call(this);
      // Cache the returned function, as required by the Intl specification (to
      // satisfy `dtf.format === dtf.format`)
      return (nativeFormat[boundFormat] ??= (date?: any) =>
        nativeFormat(date === undefined ? now() : date));
    },
    enumerable: false,
    configurable: true,
  });

  const nativeFormatToParts = proto.formatToParts;
  Object.defineProperty(proto, "formatToParts", {
    value: function formatToParts(this: any, date?: any) {
      return nativeFormatToParts.call(this, date === undefined ? now() : date);
    },
    writable: true,
    enumerable: false,
    configurable: true,
  });
}
