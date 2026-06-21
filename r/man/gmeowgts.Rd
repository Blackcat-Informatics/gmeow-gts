% SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
% SPDX-License-Identifier: MIT OR Apache-2.0

\name{gmeowgts}
\alias{gmeowgts}
\alias{abi_version}
\alias{libgts_version}
\alias{build_metadata_json}
\alias{capabilities_json}
\alias{read_json}
\alias{verify_json}
\alias{to_nquads}
\alias{from_nquads}
\alias{files_pack}
\alias{files_unpack}
\alias{files_diff_json}
\alias{status}
\alias{status_name}
\alias{unpack_flags}
\title{GTS C ABI Wrapper}
\description{
Thin R bindings for the Rust-backed \code{libgts} C ABI.
}
\usage{
abi_version()
libgts_version()
build_metadata_json()
capabilities_json()
read_json(data)
verify_json(data)
to_nquads(data)
from_nquads(text)
files_pack(paths)
files_unpack(data, destination, flags = unpack_flags[["NONE"]])
files_diff_json(data, directory)
status
status_name(code)
unpack_flags
}
\arguments{
  \item{data}{A raw vector containing a GTS archive.}
  \item{text}{A single non-\code{NA} character string containing N-Quads.}
  \item{paths}{A non-empty character vector of file or directory paths to pack.}
  \item{destination}{A single non-\code{NA} directory path for unpacked files.}
  \item{flags}{A single non-negative integer bitmask from \code{unpack_flags}.}
  \item{directory}{A single non-\code{NA} directory path to compare with an archive.}
  \item{code}{An integer C ABI status code.}
}
\format{
\code{status} and \code{unpack_flags} are named integer vectors.
}
\value{
\code{abi_version()} returns an integer ABI version. \code{libgts_version()},
\code{build_metadata_json()}, \code{capabilities_json()}, \code{read_json()},
\code{verify_json()}, \code{to_nquads()}, \code{files_unpack()}, and
\code{files_diff_json()} return character strings. \code{from_nquads()} and
\code{files_pack()} return raw vectors. \code{status_name()} returns a status
name string.
}
\details{
The wrapper copies every C ABI output buffer into an R object before releasing
it with \code{gts_buffer_free}. C ABI error handles are copied into structured
\code{gmeowgts_error} conditions and released with \code{gts_error_free}.
}
\keyword{utilities}
