/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#include "eden/scm/lib/edenfs_ffi/include/ffi.h"
#include <memory>
#include <utility>

namespace facebook::eden {

void set_matcher_promise_result(
    std::unique_ptr<MatcherPromise> matcherPromise,
    rust::Box<MercurialMatcher> matcher) {
  matcherPromise->promise.setValue(std::move(matcher));
  return;
}

void set_matcher_promise_error(
    std::unique_ptr<MatcherPromise> matcherPromise,
    rust::String error) {
  matcherPromise->promise.setException(
      std::runtime_error(std::move(error).c_str()));
  return;
}

void set_matcher_result(
    std::shared_ptr<MatcherWrapper> wrapper,
    rust::Box<::facebook::eden::MercurialMatcher> matcher) {
  wrapper->matcher_ =
      std::make_unique<rust::Box<MercurialMatcher>>(std::move(matcher));
  return;
}

void set_matcher_error(
    std::shared_ptr<MatcherWrapper> wrapper,
    rust::String error) {
  wrapper->error_ = std::move(error);
  return;
}
} // namespace facebook::eden
