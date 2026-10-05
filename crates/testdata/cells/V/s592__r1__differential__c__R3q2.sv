module top;
  localparam integer K = 5;
  if (1) begin : g
    case (2)
      K: begin : k
        localparam [Nope:0] Q = 1;
      end
      default: begin : d
        wire [3:0] w;
      end
    endcase
    localparam integer K = 2;
  end
  initial #1 $display("@bits=%0d", $bits(g.d.w));
  initial #10 $finish;
endmodule
