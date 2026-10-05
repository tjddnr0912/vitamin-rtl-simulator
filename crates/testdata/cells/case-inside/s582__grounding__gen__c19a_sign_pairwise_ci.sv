module top;
  logic signed [3:0] v; int m;
  initial begin
    v = -4'sd1; m = 9;
    case (v) inside
      -1: m = 1;
      8'h00: m = 2;
      default: m = 0;
    endcase
    $display("v=%0d m=%0d", v, m);
    v = 4'sd0; m = 9;
    case (v) inside
      -1: m = 1;
      8'h00: m = 2;
      default: m = 0;
    endcase
    $display("v=%0d m=%0d", v, m);
    #10 $finish;
  end
endmodule
