package p1;
  localparam A = (4'b1100 ==? 4'b1?00);
  localparam [64:0] PW = {64'd0, (4'b1100 ==? 4'b1?00)};
  localparam [64:0] PI = {64'd0, (8'hA5 inside {8'b1010_????})};
endpackage
module m #(parameter P = 0, parameter [64:0] Q = 0) ();
  case (1) P: begin : a initial $display("@%m P item"); end default: begin : ad initial $display("@%m P def"); end endcase
  case (1) Q: begin : b initial $display("@%m Q item"); end default: begin : bd initial $display("@%m Q def"); end endcase
endmodule
module top;
  import p1::*;
  case (1) p1::A: begin : a initial $display("@pA item"); end default: begin : ad initial $display("@pA def"); end endcase
  case (1) A: begin : b initial $display("@iA item"); end default: begin : bd initial $display("@iA def"); end endcase
  case (1) p1::PW: begin : c initial $display("@pPW item"); end default: begin : cd initial $display("@pPW def"); end endcase
  case (1) PI: begin : e initial $display("@iPI item"); end default: begin : ed initial $display("@iPI def"); end endcase
  if (1) begin : g
    localparam [64:0] GW = {64'd0, (4'b1110 !=? 4'b1?00)};
    case (1) GW: begin : f initial $display("@gGW item"); end default: begin : fd initial $display("@gGW def"); end endcase
  end
  for (genvar i = 0; i < 4; i++) begin : l
    localparam [64:0] LW = {64'd0, (i ==? 32'b????_????_????_????_????_????_????_??1?)};
    case (1) LW: begin : h initial $display("@l%0d item", i); end default: begin : hd initial $display("@l%0d def", i); end endcase
  end
  m #(.P(4'b1100 ==? 4'b1?00), .Q({64'd0, (4'b1100 ==? 4'b1?00)})) u1 ();
  m #(.P(4'b0100 ==? 4'b1?00), .Q({64'd0, (4'b0100 inside {4'b1?00})})) u0 ();
endmodule
