package p; typedef struct packed { logic a; logic [4:0] b; } st_t; localparam st_t P1 = '{a: 1'b1, b: 5'd3}; localparam st_t P2 = '{a: 1'b0, b: 5'd3}; endpackage
module m #(parameter logic [5:0] K = 0) (output logic [5:0] o); assign o = K; endmodule
module tb; import p::*; st_t v; int r; logic [5:0] o; m #(.K(P1)) u(.o(o));
  initial begin v = '{a: 1'b0, b: 5'd3}; case (v) P1: r = 1; P2: r = 2; default: r = 0; endcase
    #1 $display("DIGEST=%0d %0d %0d %0d", r, v == P2, o, o == P1); $finish; end
endmodule