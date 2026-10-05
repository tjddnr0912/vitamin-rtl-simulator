package p; typedef struct packed { logic a; logic [4:0] b; } st_t; endpackage
module m #(parameter p::st_t X = '{1'b1, 5'd3}) (output logic [7:0] o); assign o = X.b; endmodule
module tb; logic [7:0] o; m #(.X('{1'b0, 5'd7})) u(.o(o));
  initial begin #1 $display("DIGEST=%0d", o); $finish; end
endmodule