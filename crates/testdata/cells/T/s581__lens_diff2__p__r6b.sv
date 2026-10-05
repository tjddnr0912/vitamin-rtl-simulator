module m #(parameter SEL = 0);
  if (SEL) begin : arm1
    case (1)
      1: begin : n
        case (-1)
          K: begin : g wire [7:0] w = 8'd200; initial #1 $display("@m%0d a %0d bits=%0d", SEL, w, $bits(w)); end
          default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@m%0d def %0d bits=%0d", SEL, w, $bits(w)); end
        endcase
        localparam logic [31:0] K = 32'hFFFFFFFF;
      end
      default: begin : n2 initial #1 $display("@m%0d outer-default", SEL); end
    endcase
  end else begin : arm0
    case (-1)
      32'hFFFFFFFF: begin : g wire [7:0] w = 8'd200; initial #1 $display("@m%0d b %0d bits=%0d", SEL, w, $bits(w)); end
      default: begin : g wire [3:0] w = 4'd9; initial #1 $display("@m%0d bdef %0d bits=%0d", SEL, w, $bits(w)); end
    endcase
  end
endmodule
module top;
  m #(.SEL(1)) u1();
  m #(.SEL(0)) u0();
endmodule
