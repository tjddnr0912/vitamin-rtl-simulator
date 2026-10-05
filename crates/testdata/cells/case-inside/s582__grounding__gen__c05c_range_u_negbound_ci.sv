module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd0; m = 9;
    case (v) inside
      [-1:4'sd2]: m = 1;
      [-4'sd2:4'sd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd1; m = 9;
    case (v) inside
      [-1:4'sd2]: m = 1;
      [-4'sd2:4'sd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd14; m = 9;
    case (v) inside
      [-1:4'sd2]: m = 1;
      [-4'sd2:4'sd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd15; m = 9;
    case (v) inside
      [-1:4'sd2]: m = 1;
      [-4'sd2:4'sd2]: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
