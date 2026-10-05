module top;
  localparam int I = -5;
  parameter UN = 4'hA;
  localparam logic [3:0] UT = 4'hA;
  localparam UL = 4'hA;
  case (3865470567) (-(I / UN)): begin : a initial $display("@untyped-param hit"); end default: begin : ad initial $display("@untyped-param def"); end endcase
  case (3865470567) (-(I / UT)): begin : b initial $display("@typed hit"); end default: begin : bd initial $display("@typed def"); end endcase
  case (3865470567) (-(I / UL)): begin : c initial $display("@untyped-localparam hit"); end default: begin : cd initial $display("@untyped-localparam def"); end endcase
  case (3865470567) (-(I / 4'hA)): begin : e initial $display("@literal hit"); end default: begin : ed initial $display("@literal def"); end endcase
  case (-1) UN: begin : f initial $display("@UN-1 hit"); end default: begin : fd initial $display("@UN-1 def"); end endcase
  case (UN) -6: begin : g initial $display("@UNscrut m6"); end 10: begin : g2 initial $display("@UNscrut 10"); end default: begin : gd initial $display("@UNscrut def"); end endcase
  case (32'hFFFF_FFF6) -UN: begin : h initial $display("@negUN32 hit"); end default: begin : hd initial $display("@negUN32 def"); end endcase
  case (4'h6) -UN: begin : k initial $display("@negUN4 hit"); end default: begin : kd initial $display("@negUN4 def"); end endcase
  initial $display("@rt %0d %0d", -(I / UN), (3865470567 == -(I / UN)));
endmodule
