`timescale 1ns/1ns
module sub #(parameter logic [64:0] Q = 0) (); initial #1 $display("@sub %m %h", Q); endmodule
module top;
  localparam logic [64:0] W = {1'b1, 64'd3};
  case (1'b1)
    W[64]: begin : o
      localparam logic [64:0] IN = W >> 1;
      case (1)
        IN[63]: begin : i sub #(.Q(IN)) u (); end
        default: begin : i sub #(.Q(0)) u (); end
      endcase
      localparam logic [3:0] Z = IN[63] ? 4'hF : 4'h0;
      case (-1)
        Z: begin : j initial #2 $display("@j wrong-signed"); end
        15: begin : j initial #2 $display("@j 15"); end
        default: begin : j initial #2 $display("@j def"); end
      endcase
      case (15)
        Z: begin : k initial #2 $display("@k Z"); end
        default: begin : k initial #2 $display("@k def"); end
      endcase
    end
    default: begin : o initial #1 $display("@o def"); end
  endcase
endmodule
