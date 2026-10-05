`timescale 1ns/1ns
module sub #(parameter type T = logic signed [7:0]) ();
  localparam T X = -8'sd4;
  initial case (X) -4: $display("case=m4"); 12: $display("case=12"); 252: $display("case=252"); -1: $display("case=m1"); 15: $display("case=15"); default: $display("case=def"); endcase
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
