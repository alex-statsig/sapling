/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/fs/utils/ChronoParse.h"

#include <folly/Conv.h>
#include <folly/portability/GTest.h>

using namespace facebook::eden;
using namespace folly::string_piece_literals;
using namespace std::chrono_literals;
using folly::StringPiece;

TEST(Chrono, chronoErrorToString) {
  EXPECT_EQ(
      "unknown duration unit specifier",
      chronoParseErrorToString(ChronoParseError::UnknownUnit));
  EXPECT_EQ(
      "duration units must be listed from largest to smallest",
      chronoParseErrorToString(ChronoParseError::InvalidChronoUnitOrder));
  EXPECT_EQ("overflow", chronoParseErrorToString(ChronoParseError::Overflow));
  EXPECT_EQ(
      "empty input string",
      chronoParseErrorToString(ChronoParseError::EmptyInputString));
  EXPECT_EQ(
      "invalid leading character",
      chronoParseErrorToString(ChronoParseError::InvalidLeadingChar));
  EXPECT_EQ(
      "no digits found in input string",
      chronoParseErrorToString(ChronoParseError::NoDigits));
  EXPECT_EQ(
      "non-digit character found",
      chronoParseErrorToString(ChronoParseError::NonDigitChar));
  EXPECT_EQ(
      "non-whitespace character found after end of input",
      chronoParseErrorToString(ChronoParseError::NonWhitespaceAfterEnd));
  EXPECT_EQ(
      "other error", chronoParseErrorToString(ChronoParseError::OtherError));
}

TEST(Chrono, stringToDuration) {
  EXPECT_EQ(90000ms, stringToDuration("1m30s").value());
  EXPECT_EQ(90000ms, stringToDuration("1m30s  ").value());
  EXPECT_EQ(90000ms, stringToDuration("  1 m 30  s  ").value());
  EXPECT_EQ(90000ms, stringToDuration("  1\tm\n30\ts  ").value());
  EXPECT_EQ(5ns, stringToDuration("5ns").value());
  EXPECT_EQ(10s, stringToDuration("10s").value());
  EXPECT_EQ(10s, stringToDuration("10seconds").value());
  EXPECT_EQ(10s, stringToDuration("10second").value());
  EXPECT_EQ(94670856000000007ns, stringToDuration("3yr7ns").value());
  EXPECT_EQ(-10ms, stringToDuration("-10ms").value());
  EXPECT_EQ(-10ms, stringToDuration(" - 10ms").value());
  EXPECT_EQ(-38412010ms, stringToDuration("-9hr100m12s10ms").value());
}

TEST(Chrono, durationToString) {
  EXPECT_EQ("1m30s", durationToString(90000ms));
  EXPECT_EQ("1m30s", durationToString(stringToDuration("1m30s").value()));
  EXPECT_EQ("1m30s", durationToString(stringToDuration("90s").value()));
  EXPECT_EQ("-10ms", durationToString(-10ms));
  EXPECT_EQ("-10h40m12s10ms", durationToString(-38412010ms));
  EXPECT_EQ("84d", durationToString(stringToDuration("12wk").value()));
  EXPECT_EQ("84d1ns", durationToString(stringToDuration("12wk 1ns").value()));
  EXPECT_EQ("365d5h49m12s", durationToString(stringToDuration("1yr").value()));
  EXPECT_EQ("0ns", durationToString(0ms));

  EXPECT_EQ(
      "-106751d23h47m16s854ms775us808ns",
      durationToString(
          std::chrono::nanoseconds(std::numeric_limits<int64_t>::min())));
  EXPECT_EQ(
      "106751d23h47m16s854ms775us807ns",
      durationToString(
          std::chrono::nanoseconds(std::numeric_limits<int64_t>::max())));
}

namespace {
ChronoParseError stringToDurationError(StringPiece str) {
  auto result = stringToDuration(str);
  if (!result.hasError()) {
    return ChronoParseError::OtherError;
  }
  return result.error();
}
} // namespace

TEST(Chrono, stringToDurationParseErrors) {
  EXPECT_EQ(ChronoParseError::EmptyInputString, stringToDurationError(""));
  EXPECT_EQ(ChronoParseError::EmptyInputString, stringToDurationError("   "));
  EXPECT_EQ(ChronoParseError::UnknownUnit, stringToDurationError("9hr1meter"));
  EXPECT_EQ(ChronoParseError::UnknownUnit, stringToDurationError("3"));
  EXPECT_EQ(ChronoParseError::UnknownUnit, stringToDurationError("3m30"));
  EXPECT_EQ(
      ChronoParseError::InvalidChronoUnitOrder,
      stringToDurationError("10m3hr"));
  EXPECT_EQ(
      ChronoParseError::InvalidChronoUnitOrder,
      stringToDurationError("1hr2m3m"));

  // With whitespace after a valid unit followed by a negative sign
  // we correctly detect the unit name and fail with NonDigitChar.
  EXPECT_EQ(ChronoParseError::NonDigitChar, stringToDurationError("3m -10s"));
  // With no whitespace before an internal negative sign we currently detect
  // this as part of the unit name, and fail with UnknownUnit.
  EXPECT_EQ(ChronoParseError::UnknownUnit, stringToDurationError("3m-10s"));

  // The exact code that these fail with probably doesn't really matter a great
  // deal.  Check what error code they currently report just so we'll notice in
  // case it changes unexpectedly due to code changes in the future.  In general
  // though we mainly just care that these fail.
  EXPECT_EQ(
      ChronoParseError::NonDigitChar,
      stringToDurationError("1m30s plus extra garbage"));
  EXPECT_EQ(ChronoParseError::NonDigitChar, stringToDurationError("garbage"));
  EXPECT_EQ(ChronoParseError::NonDigitChar, stringToDurationError("-garbage"));
  EXPECT_EQ(
      ChronoParseError::UnknownUnit, stringToDurationError("1m\0 30s"_sp));
  EXPECT_EQ(
      ChronoParseError::NonDigitChar, stringToDurationError("1m \0 30s"_sp));
}

TEST(Chrono, stringToDurationOverflow) {
  EXPECT_EQ(ChronoParseError::Overflow, stringToDurationError("438000days"));
  EXPECT_EQ(ChronoParseError::Overflow, stringToDurationError("110000days"));
  EXPECT_EQ(ChronoParseError::Overflow, stringToDurationError("-110000days"));
}
