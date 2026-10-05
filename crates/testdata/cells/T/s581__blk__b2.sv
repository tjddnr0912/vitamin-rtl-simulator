`timescale 1ns/1ns
module t;
  localparam logic [67:0] P68 = 68'hC;
  localparam L = (P68 ==? 4'b1?00);
  localparam LN = (P68 !=? 4'b1?00);
  logic [(P68 ==? 4'b1?00) : 0] rb;
  if (P68 ==? 4'b1?00) begin : g initial $display("B1 gif then"); end else begin : h initial $display("B1 gif else"); end
  case (1) (P68 ==? 4'b1?00): begin : c initial $display("B1 gcase item"); end default: begin : d initial $display("B1 gcase default"); end endcase
endmodule
