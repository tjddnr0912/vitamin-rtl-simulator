`timescale 1ns/1ns
module t;
  localparam logic [67:0] P68 = 68'hC;
  localparam L = (P68 ==? 4'b1?00);
  localparam LN = (P68 !=? 4'b1?00);
  if (P68 ==? 4'b1?00) begin : g initial $display("gif then"); end
  else begin : h initial $display("gif else"); end
  case (1) (P68 ==? 4'b1?00): begin : c initial $display("gcase item"); end
  default: begin : d initial $display("gcase default"); end endcase
  initial #1 $display("L=%b LN=%b", L, LN);
endmodule
