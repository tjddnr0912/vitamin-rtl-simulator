module m #(parameter logic [5:0] T = 6'd3, U = 6'd4) (output logic [7:0] o);
  assign o = T + U;
endmodule
module tb; typedef logic [5:0] U;
  logic [7:0] o; m u(.o(o));
  initial begin #1 $display("DIGEST=%0d", o); $finish; end
endmodule