module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'd1; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd3; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd8; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd9; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd2; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd12; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd15; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'd4; m = 9;
    case (v) inside
      4'd1, 4'd3, [4'd8:4'd9]: m = 1;
      4'd2, 4'b11??: m = 2;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
