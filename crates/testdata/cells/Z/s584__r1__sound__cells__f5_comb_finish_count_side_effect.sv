module top;
  logic a, y;
  int fd;
  initial a = 1;
  always_comb begin y = a; if (a) begin $display("FIN t=%0t", $time); $finish; end end
  final $display("final");
endmodule
