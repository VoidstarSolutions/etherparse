use crate::*;

/// Different kinds of ICMPv6 messages.
///
/// The data stored in this enum corresponds to the statically sized data
/// at the start of an ICMPv6 packet without the checksum. If you also need
/// the checksum you can package and [`Icmpv6Type`] value in an [`Icmpv6Header`]
/// struct.
///
/// # Decoding Example (complete packet):
///
/// ```
/// # use etherparse::{PacketBuilder};
/// # let mut builder = PacketBuilder::
/// #   ethernet2([0;6], [0;6])
/// #   .ipv6([0;16], [0;16], 20)
/// #   .icmpv6_echo_request(1, 2);
/// # let payload = [1,2,3,4];
/// # let mut packet = Vec::<u8>::with_capacity(builder.size(payload.len()));
/// # builder.write(&mut packet, &payload);
/// use etherparse::PacketHeaders;
///
/// let headers = PacketHeaders::from_ethernet_slice(&packet).unwrap();
///
/// use etherparse::TransportHeader::*;
/// match headers.transport {
///     Some(Icmpv6(icmp)) => {
///         use etherparse::Icmpv6Type::*;
///         match icmp.icmp_type {
///             // Unknown is used when further decoding is currently not supported for the icmp type & code.
///             // You can still further decode the packet on your own by using the raw data in this enum
///             // together with `headers.payload` (contains the packet data after the 8th byte)
///             Unknown{ type_u8, code_u8, bytes5to8 } => println!("Unknown{{ type_u8: {}, code_u8: {}, bytes5to8: {:?} }}", type_u8, code_u8, bytes5to8),
///             DestinationUnreachable(header) => println!("{:?}", header),
///             PacketTooBig { mtu } => println!("TimeExceeded{{ mtu: {} }}", mtu),
///             TimeExceeded(code) => println!("{:?}", code),
///             ParameterProblem(header) => println!("{:?}", header),
///             EchoRequest(header) => println!("{:?}", header),
///             EchoReply(header) => println!("{:?}", header),
///         }
///     },
///     _ => {},
/// }
/// ```
///
/// # Encoding Example (only ICMPv6 part)
///
/// To get the on wire bytes of an Icmpv6Type it needs to get packaged
/// into a [`Icmpv6Header`] so the checksum gets calculated.
///
/// ```
/// # use etherparse::Ipv6Header;
/// # let ip_header: Ipv6Header = Default::default();
/// # let invoking_packet : [u8;0] = [];
///
/// use etherparse::{Icmpv6Type, icmpv6::DestUnreachableCode};
/// let t = Icmpv6Type::DestinationUnreachable(
///     DestUnreachableCode::Address
/// );
///
/// // to calculate the checksum the ip header and the payload
/// // (in case of dest unreachable the invoking packet) are needed
/// let header = t.to_header(ip_header.source, ip_header.destination, &invoking_packet).unwrap();
///
/// // an ICMPv6 packet is composed of the header and payload
/// let mut packet = Vec::with_capacity(header.header_len() + invoking_packet.len());
/// packet.extend_from_slice(&header.to_bytes());
/// packet.extend_from_slice(&invoking_packet);
/// #
/// # {
/// #   let checksum_be = header.checksum.to_be_bytes();
/// #   assert_eq!(
/// #       &packet,
/// #       &[
/// #           header.icmp_type.type_u8(),
/// #           header.icmp_type.code_u8(),
/// #           checksum_be[0],
/// #           checksum_be[1],
/// #           0,0,0,0
/// #       ]
/// #   );
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icmpv6Type {
    /// In case of an unknown icmp type is received the header elements of
    /// the first 8 bytes/octets are stored raw in this enum value.
    ///
    /// # What is part of the header for `Icmpv6Type::Unknown`?
    ///
    /// For unknown ICMPv6 type & code combination the first 8 bytes are stored
    /// in the [`Icmpv6Header`] and the rest is stored in the payload
    /// ([`Icmpv6Slice::payload`] or [`PacketHeaders::payload`]).
    ///
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |     type_u8   |    code_u8    |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |                          bytes5to8                            |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// ...                           ...                             ...  | part of payload
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    Unknown {
        /// ICMPv6 type (present in the first byte of the ICMPv6 packet).
        type_u8: u8,
        /// ICMPv6 code (present in the 2nd byte of the ICMPv6 packet).
        code_u8: u8,
        /// Bytes located at th 5th, 6th, 7th and 8th position of the ICMP packet.
        bytes5to8: [u8; 4],
    },

    /// Message sent to inform the client that the destination is unreachable for
    /// some reason.
    ///
    /// # What is part of the header for `Icmpv6Type::DestinationUnreachable`?
    ///
    /// For the `Icmpv6Type::DestinationUnreachable` type the first 8 bytes/octets of the ICMPv6
    /// packet are part of the header. The `unused` part is not stored and droped.
    /// The offending packet is stored in the payload part of the packet
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and is not part of
    /// the [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |       1       | [value as u8] |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |                           <unused>                            |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// |     <As much of invoking packet as possible without           |  | part of payload
    /// ...   the ICMPv6 packet exceeding the minimum IPv6 MTU>       ...  |
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// A Destination Unreachable message SHOULD be generated by a router, or
    /// by the IPv6 layer in the originating node, in response to a packet
    /// that cannot be delivered to its destination address for reasons other
    /// than congestion.  (An ICMPv6 message MUST NOT be generated if a
    /// packet is dropped due to congestion.)
    DestinationUnreachable(icmpv6::DestUnreachableCode),

    /// Sent if a packet to too big to be forwarded.
    ///
    /// # What is part of the header for `Icmpv6Type::PacketTooBig`?
    ///
    /// For the `Icmpv6Type::PacketTooBig` type the first 8 bytes/octets of the ICMPv6
    /// packet are part of the header. The offending packet is stored in the payload part of the packet
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and is not part of
    /// the [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |       2       |       0       |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |                              mtu                              |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// |     <As much of invoking packet as possible without           |  | part of payload
    /// ...   the ICMPv6 packet exceeding the minimum IPv6 MTU>       ...  |
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// A Packet Too Big MUST be sent by a router in response to a packet
    /// that it cannot forward because the packet is larger than the MTU of
    /// the outgoing link.  The information in this message is used as part
    /// of the Path MTU Discovery process.
    PacketTooBig {
        /// The Maximum Transmission Unit of the next-hop link.
        mtu: u32,
    },

    /// Generated when a datagram had to be discarded due to the hop limit field
    /// reaching zero.
    ///
    /// # What is part of the header for `Icmpv6Type::TimeExceeded`?
    ///
    /// For the `Icmpv6Type::TimeExceeded` type the first 8 bytes/octets of the ICMPv6
    /// packet are part of the header. The `unused` part is not stored and droped.
    /// The offending packet is stored in the payload part of the packet
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and is not part of
    /// the [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |       3       | [value as u8] |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |                           <unused>                            |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// |     <As much of invoking packet as possible without           |  | part of payload
    /// ...   the ICMPv6 packet exceeding the minimum IPv6 MTU>       ...  |
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// If a router receives a packet with a Hop Limit of zero, or if a
    /// router decrements a packet's Hop Limit to zero, it MUST discard the
    /// packet and originate an ICMPv6 Time Exceeded message with Code 0 to
    /// the source of the packet.  This indicates either a routing loop or
    /// too small an initial Hop Limit value.
    ///
    /// An ICMPv6 Time Exceeded message with Code 1 is used to report
    /// fragment reassembly timeout, as specified in [IPv6, Section 4.5].
    TimeExceeded(icmpv6::TimeExceededCode),

    /// Sent if there is a problem with a parameter in a received packet.
    ///
    /// # What is part of the header for `Icmpv6Type::ParameterProblem`?
    ///
    /// For the `Icmpv6Type::ParameterProblem` type the first 8 bytes/octets of the ICMPv6
    /// packet are part of the header. The `unused` part is not stored and droped.
    /// The offending packet is stored in the payload part of the packet
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and is not part of
    /// the [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |       4       | [value].code  |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |                        [value].pointer                        |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// |     <As much of invoking packet as possible without           |  | part of payload
    /// ...   the ICMPv6 packet exceeding the minimum IPv6 MTU>       ...  |
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// If an IPv6 node processing a packet finds a problem with a field in
    /// the IPv6 header or extension headers such that it cannot complete
    /// processing the packet, it MUST discard the packet and SHOULD
    /// originate an ICMPv6 Parameter Problem message to the packet's source,
    /// indicating the type and location of the problem.
    ParameterProblem(icmpv6::ParameterProblemHeader),

    /// Requesting an `EchoReply` from the receiver.
    ///
    /// # What is part of the header for `Icmpv6Type::EchoRequest`?
    ///
    /// For the [`Icmpv6Type::EchoRequest`] type the first 8 bytes/octets of the
    /// ICMPv6 packet are part of the header. This includes the `id` and `seq`
    /// fields. The data part of the ICMP echo request packet is part of the payload
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and not part of the
    /// [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |      128      |       0       |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |          [value].id           |         [value].seq           |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// ...                          <data>                           ...  | part of payload
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// Every node MUST implement an ICMPv6 Echo responder function that
    /// receives Echo Requests and originates corresponding Echo Replies.  A
    /// node SHOULD also implement an application-layer interface for
    /// originating Echo Requests and receiving Echo Replies, for diagnostic
    /// purposes.
    EchoRequest(IcmpEchoHeader),
    /// Response to an `EchoRequest` message.
    ///
    /// # What is part of the header for `Icmpv6Type::EchoReply`?
    ///
    /// For the [`Icmpv6Type::EchoReply`] type the first 8 bytes/octets of the
    /// ICMPv6 packet are part of the header. This includes the `id` and `seq`
    /// fields. The data part of the ICMP echo request packet is part of the payload
    /// ([`Icmpv6Slice::payload`] & [`PacketHeaders::payload`]) and not part of the
    /// [`Icmpv6Header`].
    ///
    /// ```text
    /// 0               1               2               3               4
    /// +---------------------------------------------------------------+  -
    /// |      129      |       0       |  checksum (in Icmpv6Header)   |  |
    /// +---------------------------------------------------------------+  | part of header & type
    /// |          [value].id           |         [value].seq           |  ↓
    /// +---------------------------------------------------------------+  -
    /// |                                                               |  |
    /// ...                          <data>                           ...  | part of payload
    /// |                                                               |  ↓
    /// +---------------------------------------------------------------+  -
    /// ```
    ///
    /// # RFC 4443 Description
    ///
    /// Every node MUST implement an ICMPv6 Echo responder function that
    /// receives Echo Requests and originates corresponding Echo Replies. A
    /// node SHOULD also implement an application-layer interface for
    /// originating Echo Requests and receiving Echo Replies, for diagnostic
    /// purposes.
    ///
    /// The source address of an Echo Reply sent in response to a unicast
    /// Echo Request message MUST be the same as the destination address of
    /// that Echo Request message.
    ///
    /// An Echo Reply SHOULD be sent in response to an Echo Request message
    /// sent to an IPv6 multicast or anycast address.  In this case, the
    /// source address of the reply MUST be a unicast address belonging to
    /// the interface on which the Echo Request message was received.
    ///
    /// The data received in the ICMPv6 Echo Request message MUST be returned
    /// entirely and unmodified in the ICMPv6 Echo Reply message.
    EchoReply(IcmpEchoHeader),
}

impl Icmpv6Type {
    /// Returns the type value (first byte of the ICMPv6 header) of this type.
    #[inline]
    pub fn type_u8(&self) -> u8 {
        use crate::{Icmpv6Type::*, icmpv6::*};
        match self {
            Unknown {
                type_u8,
                code_u8: _,
                bytes5to8: _,
            } => *type_u8,
            DestinationUnreachable(_) => TYPE_DST_UNREACH,
            PacketTooBig { mtu: _ } => TYPE_PACKET_TOO_BIG,
            TimeExceeded(_) => TYPE_TIME_EXCEEDED,
            ParameterProblem(_) => TYPE_PARAMETER_PROBLEM,
            EchoRequest(_) => TYPE_ECHO_REQUEST,
            EchoReply(_) => TYPE_ECHO_REPLY,
        }
    }

    /// Returns the code value (second byte of the ICMPv6 header) of this type.
    #[inline]
    pub fn code_u8(&self) -> u8 {
        use Icmpv6Type::*;
        match self {
            Unknown {
                type_u8: _,
                code_u8,
                bytes5to8: _,
            } => *code_u8,
            DestinationUnreachable(code) => code.code_u8(),
            PacketTooBig { mtu: _ } => 0,
            TimeExceeded(code) => code.code_u8(),
            ParameterProblem(header) => header.code.code_u8(),
            EchoRequest(_) => 0,
            EchoReply(_) => 0,
        }
    }

    /// Calculates the checksum of the ICMPv6 header.
    ///
    /// <p style="background:rgba(255,181,77,0.16);padding:0.75em;">
    /// <strong>Warning:</strong> Don't use this method to verfy if a checksum of a
    /// received packet is correct. This method assumes that all unused bytes are
    /// filled with zeros. If this is not the case the computed checksum value will
    /// will be incorrect for a received packet.
    ///
    /// If you want to verify that a received packet has a correct checksum use
    /// [`Icmpv6Slice::is_checksum_valid`] instead.
    /// </p>
    pub fn calc_checksum(
        &self,
        source_ip: [u8; 16],
        destination_ip: [u8; 16],
        payload: &[u8],
    ) -> Result<u16, ValueError> {
        // check that the total length fits into the field
        //
        // Note according to RFC 2460 the "Upper-Layer Packet Length" used
        // in the checksum calculation, for protocols that don't contain
        // their own length information (like ICMPv6), is "the Payload Length
        // from the IPv6 header, minus the length of any extension headers present
        // between the IPv6 header and the upper-layer header."
        let max_payload_len: usize = (std::u32::MAX as usize) - self.header_len();
        if max_payload_len < payload.len() {
            return Err(ValueError::Ipv6PayloadLengthTooLarge(payload.len()));
        }

        let msg_len = payload.len() + self.header_len();

        // calculate the checksum
        // NOTE: rfc4443 section 2.3 - Icmp6 *does* use a pseudoheader,
        // unlike Icmp4
        let pseudo_sum = checksum::Sum16BitWords::new()
            .add_16bytes(source_ip)
            .add_16bytes(destination_ip)
            .add_2bytes([0, ip_number::IPV6_ICMP])
            .add_4bytes((msg_len as u32).to_be_bytes());

        use crate::{Icmpv6Type::*, icmpv6::*};
        Ok(match self {
            Unknown {
                type_u8,
                code_u8,
                bytes5to8,
            } => pseudo_sum
                .add_2bytes([*type_u8, *code_u8])
                .add_4bytes(*bytes5to8),
            DestinationUnreachable(header) => {
                pseudo_sum.add_2bytes([TYPE_DST_UNREACH, header.code_u8()])
            }
            PacketTooBig { mtu } => pseudo_sum
                .add_2bytes([TYPE_PACKET_TOO_BIG, 0])
                .add_4bytes(mtu.to_be_bytes()),
            TimeExceeded(code) => pseudo_sum.add_2bytes([TYPE_TIME_EXCEEDED, code.code_u8()]),
            ParameterProblem(header) => pseudo_sum
                .add_2bytes([TYPE_PARAMETER_PROBLEM, header.code.code_u8()])
                .add_4bytes(header.pointer.to_be_bytes()),
            EchoRequest(echo) => pseudo_sum
                .add_2bytes([TYPE_ECHO_REQUEST, 0])
                .add_4bytes(echo.to_bytes()),
            EchoReply(echo) => pseudo_sum
                .add_2bytes([TYPE_ECHO_REPLY, 0])
                .add_4bytes(echo.to_bytes()),
        }
        .add_slice(payload)
        .ones_complement()
        .to_be())
    }

    /// Creates a header with the correct checksum.
    pub fn to_header(
        self,
        source_ip: [u8; 16],
        destination_ip: [u8; 16],
        payload: &[u8],
    ) -> Result<Icmpv6Header, ValueError> {
        Ok(Icmpv6Header {
            checksum: self.calc_checksum(source_ip, destination_ip, payload)?,
            icmp_type: self,
        })
    }

    /// Serialized length of the header in bytes/octets.
    ///
    /// Note that this size is not the size of the entire
    /// ICMPv6 packet but only the header.
    pub fn header_len(&self) -> usize {
        use Icmpv6Type::*;
        match self {
            Unknown {
                type_u8: _,
                code_u8: _,
                bytes5to8: _,
            }
            | DestinationUnreachable(_)
            | PacketTooBig { mtu: _ }
            | TimeExceeded(_)
            | ParameterProblem(_)
            | EchoRequest(_)
            | EchoReply(_) => 8,
        }
    }

    /// If the ICMP type has a fixed size returns the number of
    /// bytes that should be present after the header of this type.
    #[inline]
    pub fn fixed_payload_size(&self) -> Option<usize> {
        use Icmpv6Type::*;
        match self {
            Unknown {
                type_u8: _,
                code_u8: _,
                bytes5to8: _,
            }
            | DestinationUnreachable(_)
            | PacketTooBig { mtu: _ }
            | TimeExceeded(_)
            | ParameterProblem(_)
            | EchoRequest(_)
            | EchoReply(_) => None,
        }
    }
}