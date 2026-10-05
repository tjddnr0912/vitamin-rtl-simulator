module top;
  reg [3:0] inside; reg [3:0] x; integer m;
  initial begin
    inside = 4'b0110; x = 4'd2;
    case (x)
      inside[1:0]: m = 1;
      default: m = 0;
    endcase
    $display("A x=2 vs inside[1:0] m=%0d", m);
    x = 4'd6;
    case (x)
      inside + 4'd0, 4'd9: m = 1;
      default: m = 0;
    endcase
    $display("B x=6 vs inside+0 m=%0d", m);
    #1 $finish;
  end
endmodule
