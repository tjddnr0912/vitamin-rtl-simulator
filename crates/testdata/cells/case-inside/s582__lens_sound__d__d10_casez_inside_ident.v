module top;
  reg [3:0] inside; reg [3:0] x; integer m;
  initial begin
    inside = 4'b0110; x = 4'd2;
    casez (x)
      inside[1:0]: m = 1;
      default: m = 0;
    endcase
    $display("F casez x=2 vs inside[1:0] m=%0d", m);
    #1 $finish;
  end
endmodule
