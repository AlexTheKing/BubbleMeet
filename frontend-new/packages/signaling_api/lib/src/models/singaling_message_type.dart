enum SignalingMessageType {
  offer('Offer'),
  answer('Answer'),
  iceCandidate('ICECandidate'),
  streamControl('StreamControl'),
  ping('Ping');

  const SignalingMessageType(this.value);

  final String value;

  static SignalingMessageType? fromValue(String value) {
    return SignalingMessageType.values.firstWhere(
      (element) => element.value == value,
    );
  }
}
