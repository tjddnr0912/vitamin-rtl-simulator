`timescale 1ns/1ns
module sub #(parameter logic [64:0] Q = 0) (); initial #1 $display("@sub %m %h", Q); endmodule
module top;
  localparam logic [64:0] W = {1'b1, 64'd3};
  case (1'b1)
    W[64]: begin : o
      localparam logic [64:0] IN = W >> 1;
      case (IN[63:0])
        64'h8000_0000_0000_0001: begin : i localparam logic [3:0] Z = 4'hF; sub #(.Q(IN)) u (); end
        default: begin : i localparam logic [3:0] Z = 4'h0; sub #(.Q(0)) u (); end
      endcase
      case (-1)
        i.Z: begin : j initial #2 $display("@j hit-signed?"); end
        15: begin : j initial #2 $display("@j 15"); end
        default: begin : j initial #2 $display("@j def"); end
      endcase
    end
    default: begin : o initial #1 $display("@o def"); end
  endcase
endmodule
