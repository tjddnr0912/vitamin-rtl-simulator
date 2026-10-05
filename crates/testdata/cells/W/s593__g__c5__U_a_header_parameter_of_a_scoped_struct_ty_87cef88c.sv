package p; typedef struct packed { logic a; logic [4:0] b; } st_t; endpackage
module m #(parameter p::st_t X = '{1'b1, 5'd3}, parameter int K = 2) (output logic [7:0] o);
  assign o = X.b * K;
endmodule
module tb; import p::*;
  logic [7:0] o1, o2;
  m u1(.o(o1));
  m #(.X(6'd7), .K(3)) u2(.o(o2));
  initial begin #1 $display("DIGEST=%0d %0d", o1, o2); $finish; end
endmodule