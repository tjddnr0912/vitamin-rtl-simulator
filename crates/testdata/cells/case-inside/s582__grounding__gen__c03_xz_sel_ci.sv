module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'b1x00; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'bx100; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b011x; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b100z; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'bxxxx; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1z00; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1001; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'bzzzz; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b10x1; m = 9;
    case (v) inside
      4'b1?00: m = 1;
      4'b0110: m = 2;
      [4'd8:4'd9]: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
