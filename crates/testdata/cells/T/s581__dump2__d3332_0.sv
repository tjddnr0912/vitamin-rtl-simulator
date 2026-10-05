`timescale 1ns/1ns
module t;
  localparam real R = 1.0;
  localparam S1 = "ab";
  localparam P = 3;
  localparam [64:0] W = {1'b1, 64'd0};
  function automatic integer f(input integer x); f = x + 1; endfunction
  case (0)
    4'b1x00 & 4'b0011: begin : i_X05 initial $display("X05 item"); end
    default: begin : d_X05 initial $display("X05 default"); end
  endcase
  case (1)
    4'b1x00 & 4'b0011: begin : i_X05T initial $display("X05T item"); end
    default: begin : d_X05T initial $display("X05T default"); end
  endcase
  case (1)
    1'bx ? 1 : 1: begin : i_X20 initial $display("X20 item"); end
    default: begin : d_X20 initial $display("X20 default"); end
  endcase
  case (0)
    1'bx ? 1 : 1: begin : i_X20T initial $display("X20T item"); end
    default: begin : d_X20T initial $display("X20T default"); end
  endcase
  case (1)
    R: begin : i_L13 initial $display("L13 item"); end
    default: begin : d_L13 initial $display("L13 default"); end
  endcase
  case (2)
    R: begin : i_L13T initial $display("L13T item"); end
    default: begin : d_L13T initial $display("L13T default"); end
  endcase
  case (16'h6163)
    S1 + 1: begin : i_L07 initial $display("L07 item"); end
    default: begin : d_L07 initial $display("L07 default"); end
  endcase
  case (16'h6164)
    S1 + 1: begin : i_L07T initial $display("L07T item"); end
    default: begin : d_L07T initial $display("L07T default"); end
  endcase
  case (3)
    top.P: begin : i_L14 initial $display("L14 item"); end
    default: begin : d_L14 initial $display("L14 default"); end
  endcase
  case (4)
    top.P: begin : i_L14T initial $display("L14T item"); end
    default: begin : d_L14T initial $display("L14T default"); end
  endcase
  case (f(2))
    W: begin : i_S48 initial $display("S48 item"); end
    default: begin : d_S48 initial $display("S48 default"); end
  endcase
  case (1)
    $isunknown(4'bx100 ==? 4'b1?00): begin : i_T2 initial $display("T2 item"); end
    default: begin : d_T2 initial $display("T2 default"); end
  endcase
endmodule
