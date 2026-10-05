module top;
  wire w = 1'b1;
  logic a, b;
  always @(w) b = a2;
  always @(w) a = ~w;
  wire a2 = a;
  final $display("FIN a=%b a2=%b b=%b", a, a2, b);
endmodule
