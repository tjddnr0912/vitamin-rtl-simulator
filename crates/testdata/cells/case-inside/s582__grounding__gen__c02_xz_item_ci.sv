module top;
  logic [3:0] v; int m;
  initial begin
    v = 4'b1000; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1001; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1100; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1101; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0001; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0111; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0011; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b0000; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1010; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    v = 4'b1011; m = 9;
    case (v) inside
      4'b1x0z: m = 1;
      4'b0zz1: m = 2;
      4'b??11: m = 3;
      default: m = 0;
    endcase
    $display("v=%b m=%0d", v, m);
    #10 $finish;
  end
endmodule
